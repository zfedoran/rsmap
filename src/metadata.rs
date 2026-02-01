use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use crate::model::{CrateKind, Visibility};

/// Lightweight crate info extracted from cargo metadata (before parsing source)
#[derive(Debug, Clone)]
pub struct CrateMetadata {
    pub name: String,
    pub kind: CrateKind,
    pub edition: String,
    pub version: String,
    pub external_deps: Vec<String>,
    pub root_file: PathBuf,
    pub manifest_dir: PathBuf,
}

/// Resolve all crates in the workspace using `cargo metadata`
pub fn resolve_crates(project_path: &Path) -> Result<Vec<CrateMetadata>> {
    let manifest = project_path.join("Cargo.toml");

    // Try full metadata first; fall back to --no-deps if dependency resolution fails
    let metadata = cargo_metadata::MetadataCommand::new()
        .manifest_path(&manifest)
        .exec()
        .or_else(|_| {
            eprintln!("Full dependency resolution failed, retrying with --no-deps...");
            cargo_metadata::MetadataCommand::new()
                .manifest_path(&manifest)
                .features(cargo_metadata::CargoOpt::NoDefaultFeatures)
                .other_options(vec!["--no-deps".to_string()])
                .exec()
        })
        .context("Failed to run cargo metadata. Is this a valid Cargo project?")?;

    let workspace_members: std::collections::HashSet<_> =
        metadata.workspace_members.iter().collect();

    let mut crates = Vec::new();

    for package in &metadata.packages {
        if !workspace_members.contains(&package.id) {
            continue;
        }

        let manifest_dir = package
            .manifest_path
            .parent()
            .map(|p| PathBuf::from(p.as_std_path()))
            .unwrap_or_else(|| project_path.to_path_buf());

        // Collect external dependencies (direct only)
        let external_deps: Vec<String> = package
            .dependencies
            .iter()
            .filter(|d| d.kind == cargo_metadata::DependencyKind::Normal)
            .map(|d| d.name.clone())
            .collect();

        // Process each target in the package
        for target in &package.targets {
            let kind = if target.kind.contains(&"proc-macro".to_string()) {
                CrateKind::ProcMacro
            } else if target.kind.contains(&"lib".to_string())
                || target.kind.contains(&"rlib".to_string())
            {
                CrateKind::Lib
            } else if target.kind.contains(&"bin".to_string()) {
                CrateKind::Bin
            } else {
                continue; // skip examples, tests, benches
            };

            let root_file = PathBuf::from(target.src_path.as_std_path());

            crates.push(CrateMetadata {
                name: target.name.clone(),
                kind,
                edition: package.edition.to_string(),
                version: package.version.to_string(),
                external_deps: external_deps.clone(),
                root_file,
                manifest_dir: manifest_dir.clone(),
            });
        }
    }

    Ok(crates)
}

/// Convert syn visibility to our Visibility enum
pub fn convert_visibility(vis: &syn::Visibility) -> Visibility {
    match vis {
        syn::Visibility::Public(_) => Visibility::Pub,
        syn::Visibility::Restricted(r) => {
            let path_str = r.path.segments.iter()
                .map(|s| s.ident.to_string())
                .collect::<Vec<_>>()
                .join("::");
            match path_str.as_str() {
                "crate" => Visibility::PubCrate,
                "super" => Visibility::PubSuper,
                _ => Visibility::PubCrate, // pub(in path) treated as pub(crate)
            }
        }
        syn::Visibility::Inherited => Visibility::Private,
    }
}
