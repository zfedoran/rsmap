use anyhow::{Context, Result};
use proc_macro2::Span;
use quote::ToTokens;
use std::path::Path;
use syn;

use crate::metadata::convert_visibility;
use crate::model::{Item, ItemKind, Visibility};

/// Parse a single Rust source file and extract all top-level items
pub fn parse_file(file_path: &Path, source: &str) -> Result<Vec<Item>> {
    let syntax = syn::parse_file(source)
        .with_context(|| format!("Failed to parse {}", file_path.display()))?;

    let mut items = Vec::new();
    extract_items(&syntax.items, file_path, source, &mut items);
    Ok(items)
}

/// Extract doc comment from attributes
pub fn extract_doc_comment(attrs: &[syn::Attribute]) -> Option<String> {
    let doc_lines: Vec<String> = attrs
        .iter()
        .filter_map(|attr| {
            if attr.path().is_ident("doc") {
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
        .collect();

    if doc_lines.is_empty() {
        None
    } else {
        Some(
            doc_lines
                .iter()
                .map(|l| l.strip_prefix(' ').unwrap_or(l))
                .collect::<Vec<_>>()
                .join("\n")
                .trim()
                .to_string(),
        )
    }
}

/// Extract inner doc comments (//! style) from file attributes
pub fn extract_inner_doc_comment(attrs: &[syn::Attribute]) -> Option<String> {
    let doc_lines: Vec<String> = attrs
        .iter()
        .filter_map(|attr| {
            if attr.path().is_ident("doc") {
                if matches!(attr.style, syn::AttrStyle::Inner(_)) {
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
            }
            None
        })
        .collect();

    if doc_lines.is_empty() {
        None
    } else {
        Some(
            doc_lines
                .iter()
                .map(|l| l.strip_prefix(' ').unwrap_or(l))
                .collect::<Vec<_>>()
                .join("\n")
                .trim()
                .to_string(),
        )
    }
}

fn extract_items(
    syn_items: &[syn::Item],
    file_path: &Path,
    source: &str,
    items: &mut Vec<Item>,
) {
    for item in syn_items {
        match item {
            syn::Item::Fn(f) => {
                let sig = fn_signature(f);
                let (start, end) = span_lines(&f.sig.fn_token.span, source, item);
                items.push(Item {
                    name: f.sig.ident.to_string(),
                    kind: ItemKind::Function,
                    visibility: convert_visibility(&f.vis),
                    signature: sig,
                    doc_comment: extract_doc_comment(&f.attrs),
                    file_path: file_path.to_path_buf(),
                    line_start: start,
                    line_end: end,
                    content_hash: hash_item_source(source, start, end),
                });
            }
            syn::Item::Struct(s) => {
                let sig = struct_signature(s);
                let (start, end) = span_lines(&s.struct_token.span, source, item);
                items.push(Item {
                    name: s.ident.to_string(),
                    kind: ItemKind::Struct,
                    visibility: convert_visibility(&s.vis),
                    signature: sig,
                    doc_comment: extract_doc_comment(&s.attrs),
                    file_path: file_path.to_path_buf(),
                    line_start: start,
                    line_end: end,
                    content_hash: hash_item_source(source, start, end),
                });
            }
            syn::Item::Enum(e) => {
                let sig = enum_signature(e);
                let (start, end) = span_lines(&e.enum_token.span, source, item);
                items.push(Item {
                    name: e.ident.to_string(),
                    kind: ItemKind::Enum,
                    visibility: convert_visibility(&e.vis),
                    signature: sig,
                    doc_comment: extract_doc_comment(&e.attrs),
                    file_path: file_path.to_path_buf(),
                    line_start: start,
                    line_end: end,
                    content_hash: hash_item_source(source, start, end),
                });
            }
            syn::Item::Trait(t) => {
                let sig = trait_signature(t);
                let (start, end) = span_lines(&t.trait_token.span, source, item);
                items.push(Item {
                    name: t.ident.to_string(),
                    kind: ItemKind::Trait,
                    visibility: convert_visibility(&t.vis),
                    signature: sig,
                    doc_comment: extract_doc_comment(&t.attrs),
                    file_path: file_path.to_path_buf(),
                    line_start: start,
                    line_end: end,
                    content_hash: hash_item_source(source, start, end),
                });
            }
            syn::Item::Impl(i) => {
                let self_ty = i.self_ty.to_token_stream().to_string();
                let trait_name = i
                    .trait_
                    .as_ref()
                    .map(|(_, path, _)| path.to_token_stream().to_string());
                let sig = impl_signature(i);
                let (start, end) = span_lines(&i.impl_token.span, source, item);

                let name = if let Some(ref tn) = trait_name {
                    format!("{} for {}", tn, self_ty)
                } else {
                    self_ty.clone()
                };

                items.push(Item {
                    name,
                    kind: ItemKind::Impl {
                        self_ty,
                        trait_name,
                    },
                    visibility: Visibility::Private, // impls don't have visibility
                    signature: sig,
                    doc_comment: extract_doc_comment(&i.attrs),
                    file_path: file_path.to_path_buf(),
                    line_start: start,
                    line_end: end,
                    content_hash: hash_item_source(source, start, end),
                });
            }
            syn::Item::Type(t) => {
                let sig = format!(
                    "{}type {} = {};",
                    visibility_prefix(&t.vis),
                    t.ident,
                    t.ty.to_token_stream()
                );
                let (start, end) = span_lines(&t.type_token.span, source, item);
                items.push(Item {
                    name: t.ident.to_string(),
                    kind: ItemKind::TypeAlias,
                    visibility: convert_visibility(&t.vis),
                    signature: sig,
                    doc_comment: extract_doc_comment(&t.attrs),
                    file_path: file_path.to_path_buf(),
                    line_start: start,
                    line_end: end,
                    content_hash: hash_item_source(source, start, end),
                });
            }
            syn::Item::Const(c) => {
                let sig = format!(
                    "{}const {}: {};",
                    visibility_prefix(&c.vis),
                    c.ident,
                    c.ty.to_token_stream()
                );
                let (start, end) = span_lines(&c.const_token.span, source, item);
                items.push(Item {
                    name: c.ident.to_string(),
                    kind: ItemKind::Const,
                    visibility: convert_visibility(&c.vis),
                    signature: sig,
                    doc_comment: extract_doc_comment(&c.attrs),
                    file_path: file_path.to_path_buf(),
                    line_start: start,
                    line_end: end,
                    content_hash: hash_item_source(source, start, end),
                });
            }
            syn::Item::Static(s) => {
                let mutability = if s.mutability == syn::StaticMutability::Mut(Default::default()) {
                    "mut "
                } else {
                    ""
                };
                let sig = format!(
                    "{}static {}{}: {};",
                    visibility_prefix(&s.vis),
                    mutability,
                    s.ident,
                    s.ty.to_token_stream()
                );
                let (start, end) = span_lines(&s.static_token.span, source, item);
                items.push(Item {
                    name: s.ident.to_string(),
                    kind: ItemKind::Static,
                    visibility: convert_visibility(&s.vis),
                    signature: sig,
                    doc_comment: extract_doc_comment(&s.attrs),
                    file_path: file_path.to_path_buf(),
                    line_start: start,
                    line_end: end,
                    content_hash: hash_item_source(source, start, end),
                });
            }
            syn::Item::Macro(m) => {
                if let Some(ref ident) = m.ident {
                    let sig = format!("macro_rules! {} {{ ... }}", ident);
                    let (start, end) = span_lines(&m.mac.path.segments[0].ident.span(), source, item);
                    items.push(Item {
                        name: ident.to_string(),
                        kind: ItemKind::Macro,
                        visibility: Visibility::Private, // macro_rules are effectively pub in the crate
                        signature: sig,
                        doc_comment: extract_doc_comment(&m.attrs),
                        file_path: file_path.to_path_buf(),
                        line_start: start,
                        line_end: end,
                        content_hash: hash_item_source(source, start, end),
                    });
                }
            }
            syn::Item::Use(u) => {
                // Only record pub use (re-exports)
                if matches!(u.vis, syn::Visibility::Public(_)) {
                    let sig = format!("pub use {};", u.tree.to_token_stream());
                    let (start, end) = span_lines(&u.use_token.span, source, item);
                    items.push(Item {
                        name: use_tree_name(&u.tree),
                        kind: ItemKind::Use,
                        visibility: Visibility::Pub,
                        signature: sig,
                        doc_comment: extract_doc_comment(&u.attrs),
                        file_path: file_path.to_path_buf(),
                        line_start: start,
                        line_end: end,
                        content_hash: hash_item_source(source, start, end),
                    });
                }
            }
            _ => {}
        }
    }
}

/// Generate function signature without body
fn fn_signature(f: &syn::ItemFn) -> String {
    let vis = visibility_prefix(&f.vis);
    let asyncness = if f.sig.asyncness.is_some() {
        "async "
    } else {
        ""
    };
    let unsafety = if f.sig.unsafety.is_some() {
        "unsafe "
    } else {
        ""
    };
    let constness = if f.sig.constness.is_some() {
        "const "
    } else {
        ""
    };
    let generics = if f.sig.generics.params.is_empty() {
        String::new()
    } else {
        f.sig.generics.to_token_stream().to_string()
    };
    let where_clause = f
        .sig
        .generics
        .where_clause
        .as_ref()
        .map(|w| format!(" {}", w.to_token_stream()))
        .unwrap_or_default();

    let inputs: Vec<String> = f
        .sig
        .inputs
        .iter()
        .map(|arg| arg.to_token_stream().to_string())
        .collect();

    let output = match &f.sig.output {
        syn::ReturnType::Default => String::new(),
        syn::ReturnType::Type(_, ty) => format!(" -> {}", ty.to_token_stream()),
    };

    format!(
        "{}{}{}{}fn {}{}({}){}{};",
        vis,
        constness,
        asyncness,
        unsafety,
        f.sig.ident,
        generics,
        inputs.join(", "),
        output,
        where_clause
    )
}

/// Generate struct signature with fields
fn struct_signature(s: &syn::ItemStruct) -> String {
    let vis = visibility_prefix(&s.vis);
    let generics = if s.generics.params.is_empty() {
        String::new()
    } else {
        s.generics.to_token_stream().to_string()
    };
    let where_clause = s
        .generics
        .where_clause
        .as_ref()
        .map(|w| format!(" {}", w.to_token_stream()))
        .unwrap_or_default();

    match &s.fields {
        syn::Fields::Named(fields) => {
            let field_sigs: Vec<String> = fields
                .named
                .iter()
                .map(|f| {
                    let fvis = visibility_prefix(&f.vis);
                    let name = f.ident.as_ref().unwrap();
                    let ty = f.ty.to_token_stream();
                    format!("    {}{}: {},", fvis, name, ty)
                })
                .collect();

            format!(
                "{}struct {}{}{} {{\n{}\n}}",
                vis,
                s.ident,
                generics,
                where_clause,
                field_sigs.join("\n")
            )
        }
        syn::Fields::Unnamed(fields) => {
            let field_sigs: Vec<String> = fields
                .unnamed
                .iter()
                .map(|f| {
                    let fvis = visibility_prefix(&f.vis);
                    format!("{}{}", fvis, f.ty.to_token_stream())
                })
                .collect();

            format!(
                "{}struct {}{}({});",
                vis,
                s.ident,
                generics,
                field_sigs.join(", ")
            )
        }
        syn::Fields::Unit => {
            format!("{}struct {}{};", vis, s.ident, generics)
        }
    }
}

/// Generate enum signature with variants
fn enum_signature(e: &syn::ItemEnum) -> String {
    let vis = visibility_prefix(&e.vis);
    let generics = if e.generics.params.is_empty() {
        String::new()
    } else {
        e.generics.to_token_stream().to_string()
    };

    let variant_sigs: Vec<String> = e
        .variants
        .iter()
        .map(|v| {
            let name = &v.ident;
            match &v.fields {
                syn::Fields::Named(fields) => {
                    let fs: Vec<String> = fields
                        .named
                        .iter()
                        .map(|f| {
                            let fname = f.ident.as_ref().unwrap();
                            let ty = f.ty.to_token_stream();
                            format!("{}: {}", fname, ty)
                        })
                        .collect();
                    format!("    {} {{ {} }},", name, fs.join(", "))
                }
                syn::Fields::Unnamed(fields) => {
                    let fs: Vec<String> = fields
                        .unnamed
                        .iter()
                        .map(|f| f.ty.to_token_stream().to_string())
                        .collect();
                    format!("    {}({}),", name, fs.join(", "))
                }
                syn::Fields::Unit => format!("    {},", name),
            }
        })
        .collect();

    format!(
        "{}enum {}{} {{\n{}\n}}",
        vis,
        e.ident,
        generics,
        variant_sigs.join("\n")
    )
}

/// Generate trait signature with method signatures
fn trait_signature(t: &syn::ItemTrait) -> String {
    let vis = visibility_prefix(&t.vis);
    let unsafety = if t.unsafety.is_some() {
        "unsafe "
    } else {
        ""
    };
    let generics = if t.generics.params.is_empty() {
        String::new()
    } else {
        t.generics.to_token_stream().to_string()
    };
    let where_clause = t
        .generics
        .where_clause
        .as_ref()
        .map(|w| format!(" {}", w.to_token_stream()))
        .unwrap_or_default();

    let supertraits = if t.supertraits.is_empty() {
        String::new()
    } else {
        let bounds: Vec<String> = t
            .supertraits
            .iter()
            .map(|b| b.to_token_stream().to_string())
            .collect();
        format!(": {}", bounds.join(" + "))
    };

    let items: Vec<String> = t
        .items
        .iter()
        .filter_map(|item| match item {
            syn::TraitItem::Fn(m) => {
                let msig = trait_method_signature(m);
                Some(format!("    {}", msig))
            }
            syn::TraitItem::Type(t) => {
                let bounds = if t.bounds.is_empty() {
                    String::new()
                } else {
                    let bs: Vec<String> = t
                        .bounds
                        .iter()
                        .map(|b| b.to_token_stream().to_string())
                        .collect();
                    format!(": {}", bs.join(" + "))
                };
                Some(format!("    type {}{};", t.ident, bounds))
            }
            syn::TraitItem::Const(c) => {
                Some(format!("    const {}: {};", c.ident, c.ty.to_token_stream()))
            }
            _ => None,
        })
        .collect();

    format!(
        "{}{}trait {}{}{}{} {{\n{}\n}}",
        vis,
        unsafety,
        t.ident,
        generics,
        supertraits,
        where_clause,
        items.join("\n")
    )
}

fn trait_method_signature(m: &syn::TraitItemFn) -> String {
    let asyncness = if m.sig.asyncness.is_some() {
        "async "
    } else {
        ""
    };
    let unsafety = if m.sig.unsafety.is_some() {
        "unsafe "
    } else {
        ""
    };

    let generics = if m.sig.generics.params.is_empty() {
        String::new()
    } else {
        m.sig.generics.to_token_stream().to_string()
    };

    let inputs: Vec<String> = m
        .sig
        .inputs
        .iter()
        .map(|arg| arg.to_token_stream().to_string())
        .collect();

    let output = match &m.sig.output {
        syn::ReturnType::Default => String::new(),
        syn::ReturnType::Type(_, ty) => format!(" -> {}", ty.to_token_stream()),
    };

    format!(
        "{}{}fn {}{}({}){};",
        asyncness,
        unsafety,
        m.sig.ident,
        generics,
        inputs.join(", "),
        output
    )
}

/// Generate impl block signature with method signatures
fn impl_signature(i: &syn::ItemImpl) -> String {
    let unsafety = if i.unsafety.is_some() {
        "unsafe "
    } else {
        ""
    };
    let generics = if i.generics.params.is_empty() {
        String::new()
    } else {
        i.generics.to_token_stream().to_string()
    };
    let where_clause = i
        .generics
        .where_clause
        .as_ref()
        .map(|w| format!(" {}", w.to_token_stream()))
        .unwrap_or_default();

    let trait_part = i
        .trait_
        .as_ref()
        .map(|(bang, path, _)| {
            let neg = if bang.is_some() { "!" } else { "" };
            format!("{}{} for ", neg, path.to_token_stream())
        })
        .unwrap_or_default();

    let self_ty = i.self_ty.to_token_stream();

    let methods: Vec<String> = i
        .items
        .iter()
        .filter_map(|item| match item {
            syn::ImplItem::Fn(m) => {
                let sig = impl_method_signature(m);
                Some(format!("    {}", sig))
            }
            syn::ImplItem::Type(t) => Some(format!(
                "    type {} = {};",
                t.ident,
                t.ty.to_token_stream()
            )),
            syn::ImplItem::Const(c) => Some(format!(
                "    const {}: {};",
                c.ident,
                c.ty.to_token_stream()
            )),
            _ => None,
        })
        .collect();

    format!(
        "{}impl {}{}{}{} {{\n{}\n}}",
        unsafety,
        generics,
        trait_part,
        self_ty,
        where_clause,
        methods.join("\n")
    )
}

fn impl_method_signature(m: &syn::ImplItemFn) -> String {
    let vis = visibility_prefix(&m.vis);
    let asyncness = if m.sig.asyncness.is_some() {
        "async "
    } else {
        ""
    };
    let unsafety = if m.sig.unsafety.is_some() {
        "unsafe "
    } else {
        ""
    };

    let generics = if m.sig.generics.params.is_empty() {
        String::new()
    } else {
        m.sig.generics.to_token_stream().to_string()
    };

    let inputs: Vec<String> = m
        .sig
        .inputs
        .iter()
        .map(|arg| arg.to_token_stream().to_string())
        .collect();

    let output = match &m.sig.output {
        syn::ReturnType::Default => String::new(),
        syn::ReturnType::Type(_, ty) => format!(" -> {}", ty.to_token_stream()),
    };

    format!(
        "{}{}{}fn {}{}({}){};",
        vis,
        asyncness,
        unsafety,
        m.sig.ident,
        generics,
        inputs.join(", "),
        output
    )
}

fn visibility_prefix(vis: &syn::Visibility) -> &str {
    match vis {
        syn::Visibility::Public(_) => "pub ",
        syn::Visibility::Restricted(r) => {
            let path_str = r.path.segments.iter()
                .map(|s| s.ident.to_string())
                .collect::<Vec<_>>()
                .join("::");
            match path_str.as_str() {
                "crate" => "pub(crate) ",
                "super" => "pub(super) ",
                _ => "pub(crate) ",
            }
        }
        syn::Visibility::Inherited => "",
    }
}

fn use_tree_name(tree: &syn::UseTree) -> String {
    match tree {
        syn::UseTree::Path(p) => {
            format!("{}::{}", p.ident, use_tree_name(&p.tree))
        }
        syn::UseTree::Name(n) => n.ident.to_string(),
        syn::UseTree::Rename(r) => r.rename.to_string(),
        syn::UseTree::Glob(_) => "*".to_string(),
        syn::UseTree::Group(_) => "{...}".to_string(),
    }
}

/// Get line numbers for an item. We use a heuristic: find the span start line
/// and then count to the end of the item's token stream.
fn span_lines(keyword_span: &Span, _source: &str, item: &syn::Item) -> (usize, usize) {
    let start = keyword_span.start().line;

    // Try to get end from the item's token stream
    let tokens = item.to_token_stream();
    let mut end = start;
    for tt in tokens {
        let span = tt.span();
        let line = span.end().line;
        if line > end {
            end = line;
        }
    }

    // If we couldn't get a good end, estimate from source
    if end <= start {
        // Count lines in the token stream string as a fallback
        let item_str = item.to_token_stream().to_string();
        end = start + item_str.lines().count().saturating_sub(1);
    }

    (start, end)
}

fn hash_item_source(source: &str, line_start: usize, line_end: usize) -> String {
    let lines: Vec<&str> = source.lines().collect();
    let start = line_start.saturating_sub(1);
    let end = line_end.min(lines.len());
    let item_source = lines[start..end].join("\n");
    blake3::hash(item_source.as_bytes()).to_hex().to_string()
}

/// Hash the entire contents of a file
pub fn hash_file_contents(contents: &str) -> String {
    blake3::hash(contents.as_bytes()).to_hex().to_string()
}

/// Parse use statements from a file (for dependency analysis)
pub fn parse_use_statements(source: &str) -> Vec<String> {
    let syntax = match syn::parse_file(source) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    let mut uses = Vec::new();
    collect_use_paths(&syntax.items, &mut uses);
    uses
}

fn collect_use_paths(items: &[syn::Item], uses: &mut Vec<String>) {
    for item in items {
        match item {
            syn::Item::Use(u) => {
                collect_use_tree_paths(&u.tree, &mut String::new(), uses);
            }
            syn::Item::Mod(m) => {
                // Skip #[cfg(test)] modules
                let is_test = m.attrs.iter().any(|attr| {
                    attr.path().is_ident("cfg")
                        && attr.meta.to_token_stream().to_string().contains("test")
                });
                if !is_test {
                    if let Some((_, ref inner_items)) = m.content {
                        collect_use_paths(inner_items, uses);
                    }
                }
            }
            _ => {}
        }
    }
}

fn collect_use_tree_paths(tree: &syn::UseTree, prefix: &mut String, paths: &mut Vec<String>) {
    match tree {
        syn::UseTree::Path(p) => {
            let old_len = prefix.len();
            if !prefix.is_empty() {
                prefix.push_str("::");
            }
            prefix.push_str(&p.ident.to_string());
            collect_use_tree_paths(&p.tree, prefix, paths);
            prefix.truncate(old_len);
        }
        syn::UseTree::Name(n) => {
            let mut full_path = prefix.clone();
            if !full_path.is_empty() {
                full_path.push_str("::");
            }
            full_path.push_str(&n.ident.to_string());
            paths.push(full_path);
        }
        syn::UseTree::Rename(r) => {
            let mut full_path = prefix.clone();
            if !full_path.is_empty() {
                full_path.push_str("::");
            }
            full_path.push_str(&r.ident.to_string());
            paths.push(full_path);
        }
        syn::UseTree::Glob(_) => {
            let mut full_path = prefix.clone();
            if !full_path.is_empty() {
                full_path.push_str("::*");
            }
            paths.push(full_path);
        }
        syn::UseTree::Group(g) => {
            for tree in &g.items {
                collect_use_tree_paths(tree, prefix, paths);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_parse_function() {
        let source = r#"
pub fn hello(name: &str) -> String {
    format!("Hello, {}!", name)
}
"#;
        let items = parse_file(&PathBuf::from("test.rs"), source).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "hello");
        assert!(matches!(items[0].kind, ItemKind::Function));
        assert_eq!(items[0].visibility, Visibility::Pub);
        assert!(items[0].signature.contains("pub fn hello(name : & str) -> String"));
    }

    #[test]
    fn test_parse_struct() {
        let source = r#"
pub struct Config {
    pub name: String,
    port: u16,
}
"#;
        let items = parse_file(&PathBuf::from("test.rs"), source).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "Config");
        assert!(matches!(items[0].kind, ItemKind::Struct));
        assert!(items[0].signature.contains("pub name: String"));
        assert!(items[0].signature.contains("port: u16"));
    }

    #[test]
    fn test_parse_enum() {
        let source = r#"
pub enum Color {
    Red,
    Green,
    Blue,
    Custom(u8, u8, u8),
}
"#;
        let items = parse_file(&PathBuf::from("test.rs"), source).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "Color");
        assert!(matches!(items[0].kind, ItemKind::Enum));
    }

    #[test]
    fn test_parse_doc_comments() {
        let source = r#"
/// This is a documented function.
/// It does important things.
pub fn documented() {}
"#;
        let items = parse_file(&PathBuf::from("test.rs"), source).unwrap();
        assert_eq!(items.len(), 1);
        let doc = items[0].doc_comment.as_ref().unwrap();
        assert!(doc.contains("This is a documented function."));
        assert!(doc.contains("It does important things."));
    }

    #[test]
    fn test_parse_use_statements() {
        let source = r#"
use std::collections::HashMap;
use crate::model::{Item, Module};
use super::parse;
"#;
        let uses = parse_use_statements(source);
        assert!(uses.contains(&"std::collections::HashMap".to_string()));
        assert!(uses.contains(&"crate::model::Item".to_string()));
        assert!(uses.contains(&"crate::model::Module".to_string()));
        assert!(uses.contains(&"super::parse".to_string()));
    }
}
