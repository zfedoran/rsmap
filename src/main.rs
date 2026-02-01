#[allow(dead_code)]
mod annotations;
#[allow(dead_code)]
mod cache;
#[allow(dead_code)]
mod layer0;
#[allow(dead_code)]
mod layer1;
#[allow(dead_code)]
mod layer2;
#[allow(dead_code)]
mod layer3;
#[allow(dead_code)]
mod metadata;
#[allow(dead_code)]
mod model;
#[allow(dead_code)]
mod output;
#[allow(dead_code)]
mod parse;
#[allow(dead_code)]
mod resolve;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "rsmap")]
#[command(about = "Generate multi-layered, LLM-friendly index files for Rust codebases")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate index files (full or incremental)
    Generate {
        /// Path to the Rust project (default: current directory)
        #[arg(long, default_value = ".")]
        path: PathBuf,

        /// Output directory (default: .codebase-index/)
        #[arg(long, default_value = ".codebase-index")]
        output: PathBuf,

        /// Force full rebuild, ignoring cache
        #[arg(long)]
        no_cache: bool,
    },

    /// Manage annotations for LLM consumption
    Annotate {
        #[command(subcommand)]
        action: AnnotateAction,
    },
}

#[derive(Subcommand)]
enum AnnotateAction {
    /// Export unannotated/stale items for LLM annotation
    Export {
        /// Path to the Rust project
        #[arg(long, default_value = ".")]
        path: PathBuf,

        /// Index directory
        #[arg(long, default_value = ".codebase-index")]
        output: PathBuf,
    },

    /// Import LLM-generated annotations
    Import {
        /// Path to the TOML file with annotations
        file: PathBuf,

        /// Index directory
        #[arg(long, default_value = ".codebase-index")]
        output: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Generate {
            path,
            output,
            no_cache,
        } => run_generate(&path, &output, no_cache),
        Commands::Annotate { action } => match action {
            AnnotateAction::Export { path, output } => run_annotate_export(&path, &output),
            AnnotateAction::Import { file, output } => run_annotate_import(&file, &output),
        },
    }
}

fn run_generate(project_path: &PathBuf, output_dir: &PathBuf, no_cache: bool) -> Result<()> {
    let project_path = std::fs::canonicalize(project_path)
        .with_context(|| format!("Cannot resolve project path: {}", project_path.display()))?;

    let output_dir = if output_dir.is_relative() {
        project_path.join(output_dir)
    } else {
        output_dir.clone()
    };

    std::fs::create_dir_all(&output_dir)
        .with_context(|| format!("Cannot create output directory: {}", output_dir.display()))?;

    // Load existing cache (if any)
    let existing_cache = if no_cache {
        None
    } else {
        cache::Cache::load(&output_dir).ok()
    };

    eprintln!("Resolving cargo metadata...");
    let crate_infos =
        metadata::resolve_crates(&project_path).context("Failed to resolve cargo metadata")?;

    eprintln!(
        "Found {} crate(s): {}",
        crate_infos.len(),
        crate_infos
            .iter()
            .map(|c| c.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );

    // Parse and resolve module trees
    let mut crates = Vec::new();
    for crate_info in &crate_infos {
        eprintln!("Parsing crate: {} ({})...", crate_info.name, crate_info.kind);
        let root_module = resolve::resolve_module_tree(
            crate_info,
            &project_path,
            existing_cache.as_ref(),
        )
        .with_context(|| format!("Failed to resolve module tree for {}", crate_info.name))?;

        crates.push(model::CrateInfo {
            name: crate_info.name.clone(),
            kind: crate_info.kind.clone(),
            edition: crate_info.edition.clone(),
            version: crate_info.version.clone(),
            external_deps: crate_info.external_deps.clone(),
            root_module,
        });
    }

    // Load existing annotations
    let annotations = annotations::AnnotationStore::load(&output_dir).unwrap_or_default();

    // Generate all layers
    eprintln!("Generating Layer 0 (overview)...");
    let overview = layer0::generate_overview(&crates, &annotations);
    std::fs::write(output_dir.join("overview.md"), &overview)
        .context("Failed to write overview.md")?;

    eprintln!("Generating Layer 1 (API surface)...");
    let api_surface = layer1::generate_api_surface(&crates, &annotations);
    std::fs::write(output_dir.join("api-surface.md"), &api_surface)
        .context("Failed to write api-surface.md")?;

    eprintln!("Generating Layer 2 (relationships)...");
    let relationships = layer2::generate_relationships(&crates);
    std::fs::write(output_dir.join("relationships.md"), &relationships)
        .context("Failed to write relationships.md")?;

    eprintln!("Generating Layer 3 (JSON index)...");
    let index = layer3::generate_index(&crates);
    std::fs::write(output_dir.join("index.json"), &index)
        .context("Failed to write index.json")?;

    // Build new cache (needed for annotation staleness detection)
    eprintln!("Building cache...");
    let new_cache = cache::Cache::from_crates(&crates);

    // Update annotations (mark stale, add new entries)
    eprintln!("Updating annotations...");
    let updated_annotations = annotations::update_annotations(
        &annotations,
        &crates,
        existing_cache.as_ref(),
        &new_cache,
    );
    updated_annotations
        .save(&output_dir)
        .context("Failed to save annotations")?;

    // Save cache
    eprintln!("Saving cache...");
    new_cache
        .save(&output_dir)
        .context("Failed to save cache")?;

    eprintln!("Done! Output written to {}", output_dir.display());
    eprintln!("  - overview.md");
    eprintln!("  - api-surface.md");
    eprintln!("  - relationships.md");
    eprintln!("  - index.json");
    eprintln!("  - annotations.toml");
    eprintln!("  - cache.json");

    Ok(())
}

fn run_annotate_export(project_path: &PathBuf, output_dir: &PathBuf) -> Result<()> {
    let project_path = std::fs::canonicalize(project_path)
        .with_context(|| format!("Cannot resolve project path: {}", project_path.display()))?;

    let output_dir = if output_dir.is_relative() {
        project_path.join(output_dir)
    } else {
        output_dir.clone()
    };

    let annotations = annotations::AnnotationStore::load(&output_dir)
        .context("No annotations.toml found. Run 'generate' first.")?;

    let export = annotations::export_for_annotation(&annotations);
    println!("{}", export);

    Ok(())
}

fn run_annotate_import(file: &PathBuf, output_dir: &PathBuf) -> Result<()> {
    let output_dir = if output_dir.is_relative() {
        std::env::current_dir()?.join(output_dir)
    } else {
        output_dir.clone()
    };

    let mut annotations = annotations::AnnotationStore::load(&output_dir)
        .context("No annotations.toml found. Run 'generate' first.")?;

    let import_content =
        std::fs::read_to_string(file).with_context(|| format!("Cannot read {}", file.display()))?;

    annotations::import_annotations(&mut annotations, &import_content)
        .context("Failed to parse import file")?;

    annotations
        .save(&output_dir)
        .context("Failed to save annotations")?;

    eprintln!("Annotations imported successfully.");

    Ok(())
}
