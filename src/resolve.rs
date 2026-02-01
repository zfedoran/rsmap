use anyhow::{Context, Result};
use quote::ToTokens;
use std::path::{Path, PathBuf};

use crate::cache::Cache;
use crate::metadata::{convert_visibility, CrateMetadata};
use crate::model::{Module, Visibility};
use crate::parse;

/// Build the complete module tree for a crate
pub fn resolve_module_tree(
    crate_meta: &CrateMetadata,
    project_root: &Path,
    cache: Option<&Cache>,
) -> Result<Module> {
    let root_file = &crate_meta.root_file;
    let source = std::fs::read_to_string(root_file)
        .with_context(|| format!("Cannot read root file: {}", root_file.display()))?;

    let file_hash = parse::hash_file_contents(&source);

    // Check cache - if hash matches, we could skip parsing, but we still
    // need the module tree structure. For now, always parse but use cache
    // for staleness detection in the annotation system.
    let syntax = syn::parse_file(&source)
        .with_context(|| format!("Failed to parse {}", root_file.display()))?;

    let doc_comment = parse::extract_inner_doc_comment(&syntax.attrs);
    let items = parse::parse_file(root_file, &source)?;

    let relative_path = root_file
        .strip_prefix(project_root)
        .unwrap_or(root_file)
        .to_path_buf();

    let use_statements = parse::parse_use_statements(&source);

    let mut root_module = Module {
        path: "crate".to_string(),
        file_path: relative_path,
        file_hash,
        doc_comment,
        visibility: Visibility::Pub,
        items,
        submodules: Vec::new(),
        use_statements,
        is_inline: false,
    };

    // Resolve submodules
    resolve_submodules(
        &syntax.items,
        &mut root_module,
        root_file,
        project_root,
        cache,
    )?;

    Ok(root_module)
}

fn resolve_submodules(
    syn_items: &[syn::Item],
    parent_module: &mut Module,
    parent_file: &Path,
    project_root: &Path,
    cache: Option<&Cache>,
) -> Result<()> {
    let parent_dir = parent_file.parent().unwrap_or(Path::new("."));

    for item in syn_items {
        if let syn::Item::Mod(mod_item) = item {
            let mod_name = mod_item.ident.to_string();

            // Skip test modules
            if is_cfg_test(&mod_item.attrs) {
                continue;
            }

            let visibility = convert_visibility(&mod_item.vis);
            let doc_comment = parse::extract_doc_comment(&mod_item.attrs);

            if let Some((_, ref inner_items)) = mod_item.content {
                // Inline module: mod foo { ... }
                let source = std::fs::read_to_string(parent_file).unwrap_or_default();
                let inline_items = extract_inline_module_items(inner_items, parent_file, &source)?;

                let mod_path = format!("{}::{}", parent_module.path, mod_name);
                let relative_path = parent_file
                    .strip_prefix(project_root)
                    .unwrap_or(parent_file)
                    .to_path_buf();

                let mut sub_module = Module {
                    path: mod_path,
                    file_path: relative_path,
                    file_hash: parent_module.file_hash.clone(), // shares parent file
                    doc_comment,
                    visibility,
                    items: inline_items,
                    submodules: Vec::new(),
                    use_statements: Vec::new(), // inline modules inherit parent's scope
                    is_inline: true,
                };

                // Recursively resolve nested inline modules
                resolve_submodules(inner_items, &mut sub_module, parent_file, project_root, cache)?;

                parent_module.submodules.push(sub_module);
            } else {
                // External module: mod foo; -> look for foo.rs or foo/mod.rs
                let custom_path = get_path_attribute(&mod_item.attrs);
                let mod_file = resolve_mod_file(parent_dir, &mod_name, custom_path.as_deref())?;

                if let Some(mod_file) = mod_file {
                    let source = std::fs::read_to_string(&mod_file).with_context(|| {
                        format!("Cannot read module file: {}", mod_file.display())
                    })?;
                    let file_hash = parse::hash_file_contents(&source);

                    let syntax = syn::parse_file(&source).with_context(|| {
                        format!("Failed to parse {}", mod_file.display())
                    })?;

                    let mod_doc = parse::extract_inner_doc_comment(&syntax.attrs)
                        .or(doc_comment);
                    let items = parse::parse_file(&mod_file, &source)?;

                    let mod_path = format!("{}::{}", parent_module.path, mod_name);
                    let relative_path = mod_file
                        .strip_prefix(project_root)
                        .unwrap_or(&mod_file)
                        .to_path_buf();

                    let use_statements = parse::parse_use_statements(&source);

                    let mut sub_module = Module {
                        path: mod_path,
                        file_path: relative_path,
                        file_hash,
                        doc_comment: mod_doc,
                        visibility,
                        items,
                        submodules: Vec::new(),
                        use_statements,
                        is_inline: false,
                    };

                    // Recursively resolve
                    resolve_submodules(
                        &syntax.items,
                        &mut sub_module,
                        &mod_file,
                        project_root,
                        cache,
                    )?;

                    parent_module.submodules.push(sub_module);
                } else {
                    eprintln!(
                        "Warning: Cannot find module file for `mod {}` in {}",
                        mod_name,
                        parent_file.display()
                    );
                }
            }
        }
    }

    Ok(())
}

/// Extract items from an inline module's content
fn extract_inline_module_items(
    inner_items: &[syn::Item],
    file_path: &Path,
    _source: &str,
) -> Result<Vec<crate::model::Item>> {
    // We need to parse items directly from the syn items
    let mut items = Vec::new();
    // Re-use the same extraction logic by converting items back to source
    // This is a simplification - for inline modules we extract from the parent file's AST
    for item in inner_items {
        let item_source = item.to_token_stream().to_string();
        if let Ok(mut parsed) = parse::parse_file(file_path, &item_source) {
            items.append(&mut parsed);
        }
    }
    Ok(items)
}

/// Resolve the file path for `mod foo;` declaration
fn resolve_mod_file(
    parent_dir: &Path,
    mod_name: &str,
    custom_path: Option<&str>,
) -> Result<Option<PathBuf>> {
    if let Some(custom) = custom_path {
        let path = parent_dir.join(custom);
        if path.exists() {
            return Ok(Some(path));
        }
        return Ok(None);
    }

    // Try mod_name.rs first
    let file_path = parent_dir.join(format!("{}.rs", mod_name));
    if file_path.exists() {
        return Ok(Some(file_path));
    }

    // Try mod_name/mod.rs
    let dir_path = parent_dir.join(mod_name).join("mod.rs");
    if dir_path.exists() {
        return Ok(Some(dir_path));
    }

    Ok(None)
}

/// Check if a module has #[cfg(test)]
fn is_cfg_test(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        if attr.path().is_ident("cfg") {
            if let Ok(meta) = attr.parse_args::<syn::Ident>() {
                return meta == "test";
            }
            // Also check for cfg(test) in meta list form
            let tokens = attr.meta.to_token_stream().to_string();
            return tokens.contains("test");
        }
        false
    })
}

/// Get #[path = "..."] attribute value
fn get_path_attribute(attrs: &[syn::Attribute]) -> Option<String> {
    attrs.iter().find_map(|attr| {
        if attr.path().is_ident("path") {
            if let syn::Meta::NameValue(nv) = &attr.meta {
                if let syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(s),
                    ..
                }) = &nv.value
                {
                    return Some(s.value());
                }
            }
        }
        None
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_mod_file() {
        // This test requires actual filesystem, so we just test the logic
        let result = resolve_mod_file(Path::new("/nonexistent"), "foo", None).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_is_cfg_test() {
        let source = r#"
#[cfg(test)]
mod tests {
    fn test_something() {}
}
"#;
        let syntax = syn::parse_file(source).unwrap();
        if let syn::Item::Mod(m) = &syntax.items[0] {
            assert!(is_cfg_test(&m.attrs));
        }
    }
}
