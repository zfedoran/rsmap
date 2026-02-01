# Crate: rsmap (bin)

# crate
<!-- file: src/main.rs -->

## Types

struct Cli {
    command: Commands,
}

enum Commands {
    Generate { path: PathBuf, output: PathBuf, no_cache: bool },
    Annotate { action: AnnotateAction },
}

enum AnnotateAction {
    Export { path: PathBuf, output: PathBuf },
    Import { file: PathBuf, output: PathBuf },
}


## Functions

fn main() -> Result < () >;

fn run_generate(project_path : & PathBuf, output_dir : & PathBuf, no_cache : bool) -> Result < () >;

fn run_annotate_export(project_path : & PathBuf, output_dir : & PathBuf) -> Result < () >;

fn run_annotate_import(file : & PathBuf, output_dir : & PathBuf) -> Result < () >;


---

# crate::annotations
<!-- file: src/annotations.rs -->

## Types

/// Storage for annotations (module and item descriptions).
/// 
/// This file is LLM-facing — it contains only paths, notes, and status flags.
/// All hashes live in cache.json.
pub struct AnnotationStore {
    pub modules: BTreeMap < String , AnnotationEntry >,
    pub items: BTreeMap < String , AnnotationEntry >,
}

pub struct AnnotationEntry {
    pub note: String,
    pub stale: bool,
    pub removed: bool,
}


## Functions

fn is_false(b : & bool) -> bool;

/// Update annotations based on current crate data and cache comparison.
/// 
/// - New items get empty notes
/// - Changed items (hash differs between old and new cache) get stale=true
/// - Removed items get removed=true
pub fn update_annotations(existing : & AnnotationStore, crates : & [CrateInfo], old_cache : Option < & Cache >, new_cache : & Cache) -> AnnotationStore;

fn collect_paths(module : & Module, module_paths : & mut BTreeMap < String , () >, item_paths : & mut BTreeMap < String , () >);

/// Export unannotated or stale items for LLM annotation
pub fn export_for_annotation(annotations : & AnnotationStore) -> String;

/// Import annotations from a TOML string (typically LLM-generated)
pub fn import_annotations(store : & mut AnnotationStore, import_content : & str) -> Result < () >;


## Impl AnnotationStore

impl AnnotationStore {
    pub fn load(output_dir : & Path) -> Result < Self >;
    pub fn save(& self, output_dir : & Path) -> Result < () >;
}


---

# crate::cache
<!-- file: src/cache.rs -->

## Types

/// Cache of all hashes for incremental rebuilds and staleness detection.
/// 
/// This is the single source of truth for change detection. LLM-facing files
/// (annotations.toml, api-surface.md, etc.) never contain hashes.
pub struct Cache {
    pub files: BTreeMap < String , CacheFileEntry >,
    pub modules: BTreeMap < String , String >,
    pub items: BTreeMap < String , String >,
}

pub struct CacheFileEntry {
    pub hash: String,
    pub last_indexed: String,
}


## Functions

fn collect_hashes(module : & Module, cache : & mut Cache, now : & str);


## Impl Cache

impl Cache {
    pub fn load(output_dir : & Path) -> Result < Self >;
    pub fn save(& self, output_dir : & Path) -> Result < () >;
    pub fn from_crates(crates : & [CrateInfo]) -> Self;
    pub fn is_file_unchanged(& self, file_path : & str, current_hash : & str) -> bool;
    pub fn module_hash_changed(& self, other : & Cache, module_path : & str) -> bool;
    pub fn item_hash_changed(& self, other : & Cache, item_path : & str) -> bool;
}


---

# crate::layer0
<!-- file: src/layer0.rs -->

## Functions

/// Generate Layer 0: Overview (overview.md)
/// 
/// Contains crate info, module tree with descriptions, and token estimates.
pub fn generate_overview(crates : & [CrateInfo], annotations : & AnnotationStore) -> String;

fn write_module_tree(out : & mut String, module : & Module, depth : usize, annotations : & AnnotationStore);

/// Get module description from various sources (priority order):
/// 1. Inner doc comment (//!)
/// 2. Annotation
/// 3. Empty placeholder
fn get_module_description(module : & Module, annotations : & AnnotationStore) -> String;


---

# crate::layer1
<!-- file: src/layer1.rs -->

## Functions

/// Generate Layer 1: API Surface (api-surface.md)
/// 
/// All items (pub AND private), grouped by module, signatures only.
pub fn generate_api_surface(crates : & [CrateInfo], annotations : & AnnotationStore) -> String;

fn write_module_surface(out : & mut String, module : & Module, annotations : & AnnotationStore);

fn write_item(out : & mut String, item : & Item, annotations : & AnnotationStore, module_path : & str);

fn format_impl_header(kind : & ItemKind) -> String;


---

# crate::layer2
<!-- file: src/layer2.rs -->

## Functions

/// Generate Layer 2: Relationships (relationships.md)
/// 
/// Includes trait implementation map, error chains, module dependencies,
/// and type usage hotspots.
pub fn generate_relationships(crates : & [CrateInfo]) -> String;

fn collect_relationships(module : & Module, trait_impls : & mut BTreeMap < String , BTreeSet < String > >, from_impls : & mut BTreeSet < (String , String) >, module_deps : & mut BTreeMap < String , BTreeSet < String > >, type_usage : & mut BTreeMap < String , BTreeSet < String > >);

/// Clean a type name by removing generics and whitespace
fn clean_type_name(name : & str) -> String;

/// Extract the source type from a From<T> trait name
fn extract_from_source(trait_str : & str) -> Option < String >;

/// Build error chain strings from From impls
fn build_error_chains(from_impls : & [(String , String)]) -> Vec < String >;

/// Follow a chain from current node to its end, outputting the complete chain
fn follow_chain(graph : & HashMap < String , Vec < String > >, current : & str, chain : & mut Vec < String >, visited : & mut HashSet < String >, results : & mut Vec < String >);

/// Extract internal module dependency from a use path
fn extract_internal_module_dep(use_path : & str) -> Option < String >;

/// Extract type names from a signature string (heuristic)
fn extract_type_names_from_signature(sig : & str) -> Vec < String >;

fn is_keyword(word : & str) -> bool;


---

# crate::layer3
<!-- file: src/layer3.rs -->

## Types

/// An entry in the JSON lookup index
struct IndexEntry {
    file: String,
    line_start: usize,
    line_end: usize,
    kind: String,
    visibility: String,
}


## Functions

/// Generate Layer 3: JSON Lookup Index (index.json)
/// 
/// A lookup table mapping fully-qualified item paths to their file locations
/// and line ranges. Designed for tooling to fetch specific source ranges.
pub fn generate_index(crates : & [CrateInfo]) -> String;

fn collect_index_entries(module : & Module, index : & mut BTreeMap < String , IndexEntry >);

fn item_full_path(module_path : & str, item : & Item) -> String;


---

# crate::metadata
<!-- file: src/metadata.rs -->

## Types

/// Lightweight crate info extracted from cargo metadata (before parsing source)
pub struct CrateMetadata {
    pub name: String,
    pub kind: CrateKind,
    pub edition: String,
    pub version: String,
    pub external_deps: Vec < String >,
    pub root_file: PathBuf,
    pub manifest_dir: PathBuf,
}


## Functions

/// Resolve all crates in the workspace using `cargo metadata`
pub fn resolve_crates(project_path : & Path) -> Result < Vec < CrateMetadata > >;

/// Convert syn visibility to our Visibility enum
pub fn convert_visibility(vis : & syn :: Visibility) -> Visibility;


---

# crate::model
<!-- file: src/model.rs -->

## Types

pub struct CrateInfo {
    pub name: String,
    pub kind: CrateKind,
    pub edition: String,
    pub version: String,
    pub external_deps: Vec < String >,
    pub root_module: Module,
}

pub enum CrateKind {
    Bin,
    Lib,
    ProcMacro,
}

pub struct Module {
    pub path: String,
    pub file_path: PathBuf,
    pub file_hash: String,
    pub doc_comment: Option < String >,
    pub visibility: Visibility,
    pub items: Vec < Item >,
    pub submodules: Vec < Module >,
    pub use_statements: Vec < String >,
    pub is_inline: bool,
}

pub struct Item {
    pub name: String,
    pub kind: ItemKind,
    pub visibility: Visibility,
    pub signature: String,
    pub doc_comment: Option < String >,
    pub file_path: PathBuf,
    pub line_start: usize,
    pub line_end: usize,
    pub content_hash: String,
}

pub enum ItemKind {
    Function,
    Struct,
    Enum,
    Trait,
    Impl { self_ty: String, trait_name: Option < String > },
    TypeAlias,
    Const,
    Static,
    Macro,
    Use,
}

pub enum Visibility {
    Pub,
    PubCrate,
    PubSuper,
    Private,
}


## Impl std :: fmt :: Display for CrateKind

impl std :: fmt :: Display for CrateKind {
    fn fmt(& self, f : & mut std :: fmt :: Formatter < '_ >) -> std :: fmt :: Result;
}


## Impl std :: fmt :: Display for ItemKind

impl std :: fmt :: Display for ItemKind {
    fn fmt(& self, f : & mut std :: fmt :: Formatter < '_ >) -> std :: fmt :: Result;
}


## Impl std :: fmt :: Display for Visibility

impl std :: fmt :: Display for Visibility {
    fn fmt(& self, f : & mut std :: fmt :: Formatter < '_ >) -> std :: fmt :: Result;
}


## Impl Visibility

impl Visibility {
    pub fn prefix(& self) -> & str;
}


## Impl Module

impl Module {
    pub fn all_items(& self) -> Vec < & Item >;
    pub fn all_modules(& self) -> Vec < & Module >;
    pub fn short_name(& self) -> & str;
}


---

# crate::output
<!-- file: src/output.rs -->

## Functions

/// Markdown/text formatting utilities
/// Indent every line of text by the given number of spaces
pub fn indent(text : & str, spaces : usize) -> String;

/// Format a module path as a tree entry with indentation
pub fn tree_entry(path : & str, description : & str, depth : usize) -> String;

/// Strip the "crate::" prefix from a module path for display
pub fn display_module_path(path : & str) -> & str;

/// Truncate a string to a maximum length, adding "..." if truncated
pub fn truncate(s : & str, max_len : usize) -> String;

/// Format a code block in markdown
pub fn code_block(code : & str, language : & str) -> String;


---

# crate::parse
<!-- file: src/parse.rs -->

## Functions

/// Parse a single Rust source file and extract all top-level items
pub fn parse_file(file_path : & Path, source : & str) -> Result < Vec < Item > >;

/// Extract doc comment from attributes
pub fn extract_doc_comment(attrs : & [syn :: Attribute]) -> Option < String >;

/// Extract inner doc comments (//! style) from file attributes
pub fn extract_inner_doc_comment(attrs : & [syn :: Attribute]) -> Option < String >;

fn extract_items(syn_items : & [syn :: Item], file_path : & Path, source : & str, items : & mut Vec < Item >);

/// Generate function signature without body
fn fn_signature(f : & syn :: ItemFn) -> String;

/// Generate struct signature with fields
fn struct_signature(s : & syn :: ItemStruct) -> String;

/// Generate enum signature with variants
fn enum_signature(e : & syn :: ItemEnum) -> String;

/// Generate trait signature with method signatures
fn trait_signature(t : & syn :: ItemTrait) -> String;

fn trait_method_signature(m : & syn :: TraitItemFn) -> String;

/// Generate impl block signature with method signatures
fn impl_signature(i : & syn :: ItemImpl) -> String;

fn impl_method_signature(m : & syn :: ImplItemFn) -> String;

fn visibility_prefix(vis : & syn :: Visibility) -> & str;

fn use_tree_name(tree : & syn :: UseTree) -> String;

/// Get line numbers for an item. We use a heuristic: find the span start line
/// and then count to the end of the item's token stream.
fn span_lines(keyword_span : & Span, _source : & str, item : & syn :: Item) -> (usize , usize);

fn hash_item_source(source : & str, line_start : usize, line_end : usize) -> String;

/// Hash the entire contents of a file
pub fn hash_file_contents(contents : & str) -> String;

/// Parse use statements from a file (for dependency analysis)
pub fn parse_use_statements(source : & str) -> Vec < String >;

fn collect_use_paths(items : & [syn :: Item], uses : & mut Vec < String >);

fn collect_use_tree_paths(tree : & syn :: UseTree, prefix : & mut String, paths : & mut Vec < String >);


---

# crate::resolve
<!-- file: src/resolve.rs -->

## Functions

/// Build the complete module tree for a crate
pub fn resolve_module_tree(crate_meta : & CrateMetadata, project_root : & Path, cache : Option < & Cache >) -> Result < Module >;

fn resolve_submodules(syn_items : & [syn :: Item], parent_module : & mut Module, parent_file : & Path, project_root : & Path, cache : Option < & Cache >) -> Result < () >;

/// Extract items from an inline module's content
fn extract_inline_module_items(inner_items : & [syn :: Item], file_path : & Path, _source : & str) -> Result < Vec < crate :: model :: Item > >;

/// Resolve the file path for `mod foo;` declaration
fn resolve_mod_file(parent_dir : & Path, mod_name : & str, custom_path : Option < & str >) -> Result < Option < PathBuf > >;

/// Check if a module has #[cfg(test)]
fn is_cfg_test(attrs : & [syn :: Attribute]) -> bool;

/// Get #[path = "..."] attribute value
fn get_path_attribute(attrs : & [syn :: Attribute]) -> Option < String >;


---

