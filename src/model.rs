use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrateInfo {
    pub name: String,
    pub kind: CrateKind,
    pub edition: String,
    pub version: String,
    pub external_deps: Vec<String>,
    pub root_module: Module,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CrateKind {
    Bin,
    Lib,
    ProcMacro,
}

impl std::fmt::Display for CrateKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CrateKind::Bin => write!(f, "bin"),
            CrateKind::Lib => write!(f, "lib"),
            CrateKind::ProcMacro => write!(f, "proc-macro"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Module {
    /// Module path, e.g. "crate::engine::eval"
    pub path: String,
    pub file_path: PathBuf,
    pub file_hash: String,
    pub doc_comment: Option<String>,
    pub visibility: Visibility,
    pub items: Vec<Item>,
    pub submodules: Vec<Module>,
    /// Use statements found in this module (for dependency analysis)
    pub use_statements: Vec<String>,
    /// Whether this is an inline module (mod foo { ... })
    pub is_inline: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub name: String,
    pub kind: ItemKind,
    pub visibility: Visibility,
    /// Signature text with body stripped
    pub signature: String,
    pub doc_comment: Option<String>,
    pub file_path: PathBuf,
    pub line_start: usize,
    pub line_end: usize,
    /// Hash of the item's full source text
    pub content_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ItemKind {
    Function,
    Struct,
    Enum,
    Trait,
    Impl {
        self_ty: String,
        trait_name: Option<String>,
    },
    TypeAlias,
    Const,
    Static,
    Macro,
    /// Re-exports only (pub use)
    Use,
}

impl std::fmt::Display for ItemKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ItemKind::Function => write!(f, "function"),
            ItemKind::Struct => write!(f, "struct"),
            ItemKind::Enum => write!(f, "enum"),
            ItemKind::Trait => write!(f, "trait"),
            ItemKind::Impl {
                self_ty,
                trait_name,
            } => {
                if let Some(t) = trait_name {
                    write!(f, "impl {} for {}", t, self_ty)
                } else {
                    write!(f, "impl {}", self_ty)
                }
            }
            ItemKind::TypeAlias => write!(f, "type_alias"),
            ItemKind::Const => write!(f, "const"),
            ItemKind::Static => write!(f, "static"),
            ItemKind::Macro => write!(f, "macro"),
            ItemKind::Use => write!(f, "use"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Visibility {
    Pub,
    PubCrate,
    PubSuper,
    Private,
}

impl std::fmt::Display for Visibility {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Visibility::Pub => write!(f, "pub"),
            Visibility::PubCrate => write!(f, "pub(crate)"),
            Visibility::PubSuper => write!(f, "pub(super)"),
            Visibility::Private => write!(f, "private"),
        }
    }
}

impl Visibility {
    /// Returns the prefix to use in output, or empty string for private
    pub fn prefix(&self) -> &str {
        match self {
            Visibility::Pub => "pub ",
            Visibility::PubCrate => "pub(crate) ",
            Visibility::PubSuper => "pub(super) ",
            Visibility::Private => "",
        }
    }
}

impl Module {
    /// Recursively collect all items across this module and submodules
    pub fn all_items(&self) -> Vec<&Item> {
        let mut result: Vec<&Item> = self.items.iter().collect();
        for sub in &self.submodules {
            result.extend(sub.all_items());
        }
        result
    }

    /// Recursively collect all modules (including self)
    pub fn all_modules(&self) -> Vec<&Module> {
        let mut result = vec![self];
        for sub in &self.submodules {
            result.extend(sub.all_modules());
        }
        result
    }

    /// Get the short name of this module (last segment of path)
    pub fn short_name(&self) -> &str {
        self.path.rsplit("::").next().unwrap_or(&self.path)
    }
}
