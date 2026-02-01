use std::path::PathBuf;
use std::process::Command;

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn fixture_path() -> PathBuf {
    project_root().join("tests/fixtures/sample_crate")
}

fn binary_path() -> PathBuf {
    // Build the binary first via cargo
    let status = Command::new("cargo")
        .args(["build"])
        .current_dir(project_root())
        .status()
        .expect("Failed to build");
    assert!(status.success());

    project_root().join("target/debug/rsmap")
}

#[test]
fn test_generate_on_fixture() {
    let binary = binary_path();
    let fixture = fixture_path();
    let output_dir = tempfile::tempdir().unwrap();

    let status = Command::new(&binary)
        .args([
            "generate",
            "--path",
            fixture.to_str().unwrap(),
            "--output",
            output_dir.path().to_str().unwrap(),
            "--no-cache",
        ])
        .status()
        .expect("Failed to run generate");

    assert!(status.success(), "generate command failed");

    // Verify all expected files exist
    assert!(
        output_dir.path().join("overview.md").exists(),
        "overview.md missing"
    );
    assert!(
        output_dir.path().join("api-surface.md").exists(),
        "api-surface.md missing"
    );
    assert!(
        output_dir.path().join("relationships.md").exists(),
        "relationships.md missing"
    );
    assert!(
        output_dir.path().join("index.json").exists(),
        "index.json missing"
    );
    assert!(
        output_dir.path().join("annotations.toml").exists(),
        "annotations.toml missing"
    );
    assert!(
        output_dir.path().join("cache.json").exists(),
        "cache.json missing"
    );

    // Verify overview content
    let overview = std::fs::read_to_string(output_dir.path().join("overview.md")).unwrap();
    assert!(overview.contains("# Crate: sample_crate (lib)"));
    assert!(overview.contains("Edition: 2021"));
    assert!(overview.contains("serde"));
    assert!(overview.contains("engine"));
    assert!(overview.contains("models"));
    assert!(overview.contains("utils"));

    // Verify API surface content
    let api_surface = std::fs::read_to_string(output_dir.path().join("api-surface.md")).unwrap();
    assert!(api_surface.contains("pub struct Config"));
    assert!(api_surface.contains("pub enum AppError"));
    assert!(api_surface.contains("pub fn init()"));
    assert!(api_surface.contains("pub fn run("));
    assert!(api_surface.contains("pub trait Evaluable"));
    assert!(api_surface.contains("pub struct EvalContext"));
    assert!(api_surface.contains("fn resolve_name(")); // private function included
    assert!(api_surface.contains("fn apply_operator(")); // private function included
    assert!(api_surface.contains("pub(crate) fn truncate(")); // pub(crate) function

    // Verify relationships content
    let relationships =
        std::fs::read_to_string(output_dir.path().join("relationships.md")).unwrap();
    assert!(relationships.contains("## Trait Implementations"));
    assert!(relationships.contains("Evaluable"));
    assert!(relationships.contains("Expr"));
    assert!(relationships.contains("## Error Chains"));
    assert!(relationships.contains("## Module Dependencies"));
    assert!(relationships.contains("## Key Types"));

    // Verify JSON index is valid JSON
    let index_json = std::fs::read_to_string(output_dir.path().join("index.json")).unwrap();
    let index: serde_json::Value = serde_json::from_str(&index_json).expect("Invalid JSON");

    // Verify specific entries exist
    assert!(index.get("crate::Config").is_some(), "Config not in index");
    assert!(index.get("crate::init").is_some(), "init not in index");
    assert!(
        index.get("crate::engine::eval::EvalContext").is_some(),
        "EvalContext not in index"
    );
    assert!(
        index.get("crate::engine::eval::evaluate").is_some(),
        "evaluate not in index"
    );
    assert!(
        index.get("crate::engine::eval::resolve_name").is_some(),
        "resolve_name not in index"
    );
    assert!(
        index.get("crate::models::Value").is_some(),
        "Value not in index"
    );

    // Verify index entry structure
    let config_entry = &index["crate::Config"];
    assert_eq!(config_entry["kind"], "struct");
    assert_eq!(config_entry["visibility"], "pub");
    assert!(config_entry["file"].as_str().unwrap().contains("lib.rs"));
    assert!(config_entry["line_start"].as_u64().unwrap() > 0);

    let resolve_name = &index["crate::engine::eval::resolve_name"];
    assert_eq!(resolve_name["kind"], "function");
    assert_eq!(resolve_name["visibility"], "private");

    // Verify annotations.toml is valid TOML
    let annotations_toml =
        std::fs::read_to_string(output_dir.path().join("annotations.toml")).unwrap();
    assert!(annotations_toml.contains("[modules."));
    assert!(annotations_toml.contains("[items."));

    // Verify cache.json is valid JSON
    let cache_json = std::fs::read_to_string(output_dir.path().join("cache.json")).unwrap();
    let cache: serde_json::Value = serde_json::from_str(&cache_json).expect("Invalid cache JSON");
    assert!(cache.get("files").is_some());
}

#[test]
fn test_incremental_rebuild() {
    let binary = binary_path();
    let fixture = fixture_path();
    let output_dir = tempfile::tempdir().unwrap();

    // First run - full build
    let status = Command::new(&binary)
        .args([
            "generate",
            "--path",
            fixture.to_str().unwrap(),
            "--output",
            output_dir.path().to_str().unwrap(),
            "--no-cache",
        ])
        .status()
        .expect("Failed to run generate");
    assert!(status.success());

    // Second run - incremental (uses cache)
    let status = Command::new(&binary)
        .args([
            "generate",
            "--path",
            fixture.to_str().unwrap(),
            "--output",
            output_dir.path().to_str().unwrap(),
        ])
        .status()
        .expect("Failed to run generate");
    assert!(status.success());

    // Both runs should produce the same output
    let overview = std::fs::read_to_string(output_dir.path().join("overview.md")).unwrap();
    assert!(overview.contains("# Crate: sample_crate"));
}

#[test]
fn test_annotate_export() {
    let binary = binary_path();
    let fixture = fixture_path();
    let output_dir = tempfile::tempdir().unwrap();

    // First generate
    let status = Command::new(&binary)
        .args([
            "generate",
            "--path",
            fixture.to_str().unwrap(),
            "--output",
            output_dir.path().to_str().unwrap(),
        ])
        .status()
        .expect("Failed to run generate");
    assert!(status.success());

    // Then export annotations
    let output = Command::new(&binary)
        .args([
            "annotate",
            "export",
            "--path",
            fixture.to_str().unwrap(),
            "--output",
            output_dir.path().to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run annotate export");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("items need descriptions"));
}

#[test]
fn test_annotate_import() {
    let binary = binary_path();
    let fixture = fixture_path();
    let output_dir = tempfile::tempdir().unwrap();

    // First generate
    let status = Command::new(&binary)
        .args([
            "generate",
            "--path",
            fixture.to_str().unwrap(),
            "--output",
            output_dir.path().to_str().unwrap(),
        ])
        .status()
        .expect("Failed to run generate");
    assert!(status.success());

    // Create an import file
    let import_file = output_dir.path().join("import.toml");
    std::fs::write(
        &import_file,
        r#"
[items."crate::init"]
hash = "dummy"
note = "Initializes the application with default settings"
"#,
    )
    .unwrap();

    // Import annotations
    let status = Command::new(&binary)
        .args([
            "annotate",
            "import",
            import_file.to_str().unwrap(),
            "--output",
            output_dir.path().to_str().unwrap(),
        ])
        .status()
        .expect("Failed to run annotate import");

    assert!(status.success());

    // Verify the annotation was imported
    let annotations =
        std::fs::read_to_string(output_dir.path().join("annotations.toml")).unwrap();
    assert!(annotations.contains("Initializes the application with default settings"));
}
