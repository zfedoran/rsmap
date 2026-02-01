use serde::Serialize;
use std::collections::BTreeMap;

use crate::model::{CrateInfo, Item, ItemKind, Module, Visibility};

/// An entry in the JSON lookup index
#[derive(Debug, Serialize)]
struct IndexEntry {
    file: String,
    line_start: usize,
    line_end: usize,
    kind: String,
    visibility: String,
}

/// Generate Layer 3: JSON Lookup Index (index.json)
///
/// A lookup table mapping fully-qualified item paths to their file locations
/// and line ranges. Designed for tooling to fetch specific source ranges.
pub fn generate_index(crates: &[CrateInfo]) -> String {
    let mut index: BTreeMap<String, IndexEntry> = BTreeMap::new();

    for crate_info in crates {
        collect_index_entries(&crate_info.root_module, &mut index);
    }

    serde_json::to_string_pretty(&index).unwrap_or_else(|_| "{}".to_string())
}

fn collect_index_entries(module: &Module, index: &mut BTreeMap<String, IndexEntry>) {
    for item in &module.items {
        let full_path = item_full_path(&module.path, item);
        let kind_str = match &item.kind {
            ItemKind::Function => "function".to_string(),
            ItemKind::Struct => "struct".to_string(),
            ItemKind::Enum => "enum".to_string(),
            ItemKind::Trait => "trait".to_string(),
            ItemKind::Impl {
                self_ty,
                trait_name,
            } => {
                if let Some(tn) = trait_name {
                    format!("impl {} for {}", tn, self_ty)
                } else {
                    format!("impl {}", self_ty)
                }
            }
            ItemKind::TypeAlias => "type_alias".to_string(),
            ItemKind::Const => "const".to_string(),
            ItemKind::Static => "static".to_string(),
            ItemKind::Macro => "macro".to_string(),
            ItemKind::Use => "use".to_string(),
        };

        let vis_str = match item.visibility {
            Visibility::Pub => "pub",
            Visibility::PubCrate => "pub(crate)",
            Visibility::PubSuper => "pub(super)",
            Visibility::Private => "private",
        };

        index.insert(
            full_path,
            IndexEntry {
                file: module.file_path.display().to_string(),
                line_start: item.line_start,
                line_end: item.line_end,
                kind: kind_str,
                visibility: vis_str.to_string(),
            },
        );
    }

    for sub in &module.submodules {
        collect_index_entries(sub, index);
    }
}

fn item_full_path(module_path: &str, item: &Item) -> String {
    match &item.kind {
        ItemKind::Impl {
            self_ty,
            trait_name,
        } => {
            if let Some(tn) = trait_name {
                format!("{}::impl {} for {}", module_path, tn, self_ty)
            } else {
                format!("{}::impl {}", module_path, self_ty)
            }
        }
        _ => format!("{}::{}", module_path, item.name),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;
    use std::path::PathBuf;

    #[test]
    fn test_generate_index() {
        let crates = vec![CrateInfo {
            name: "test".to_string(),
            kind: CrateKind::Lib,
            edition: "2021".to_string(),
            version: "0.1.0".to_string(),
            external_deps: vec![],
            root_module: Module {
                path: "crate".to_string(),
                file_path: PathBuf::from("src/lib.rs"),
                file_hash: "abc".to_string(),
                doc_comment: None,
                visibility: Visibility::Pub,
                items: vec![
                    Item {
                        name: "Config".to_string(),
                        kind: ItemKind::Struct,
                        visibility: Visibility::Pub,
                        signature: "pub struct Config {}".to_string(),
                        doc_comment: None,
                        file_path: PathBuf::from("src/lib.rs"),
                        line_start: 1,
                        line_end: 5,
                        content_hash: "h1".to_string(),
                    },
                    Item {
                        name: "init".to_string(),
                        kind: ItemKind::Function,
                        visibility: Visibility::Pub,
                        signature: "pub fn init();".to_string(),
                        doc_comment: None,
                        file_path: PathBuf::from("src/lib.rs"),
                        line_start: 7,
                        line_end: 15,
                        content_hash: "h2".to_string(),
                    },
                ],
                submodules: vec![],
                use_statements: vec![],
                is_inline: false,
            },
        }];

        let json = generate_index(&crates);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert!(parsed.get("crate::Config").is_some());
        assert!(parsed.get("crate::init").is_some());

        let config = &parsed["crate::Config"];
        assert_eq!(config["kind"], "struct");
        assert_eq!(config["visibility"], "pub");
        assert_eq!(config["line_start"], 1);
        assert_eq!(config["line_end"], 5);
    }
}
