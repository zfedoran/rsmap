---
name: rsmap
trigger: /rsmap
---

You are an assistant that uses rsmap to index and explore Rust codebases.

rsmap generates a multi-layered, LLM-friendly index of Rust projects.

## Commands

Generate an index:
  rsmap generate --path <RUST_PROJECT> --output <OUTPUT_DIR>

Export unannotated items for LLM annotation:
  rsmap annotate export --path <PROJECT> --output <INDEX_DIR>

Import annotations:
  rsmap annotate import <FILE> --output <INDEX_DIR>

Use --no-cache to force a full rebuild.

## Output Files

The index has 4 layers plus metadata:

- overview.md       — Layer 0: Crate metadata, module tree with descriptions
- api-surface.md    — Layer 1: Full public/private API signatures grouped by module
- relationships.md  — Layer 2: Trait impls, error chains, module deps, type hotspots
- index.json        — Layer 3: Fully-qualified path → file location lookup table
- annotations.toml  — LLM-facing descriptions (note/stale/removed per item)
- cache.json        — BLAKE3 hashes for incremental rebuilds

## How to Use the Index

When the user asks about a Rust codebase that has an rsmap index:

1. Start with overview.md to understand crate structure and module tree
2. Use api-surface.md to find specific types, functions, traits, and impls
3. Use relationships.md to understand cross-cutting concerns (trait impls,
   dependency flow, type hotspots)
4. Use index.json to look up exact file paths and line ranges, then read
   the actual source when needed

## Annotation Workflow

When asked to annotate a codebase:
1. Run `annotate export` to get unannotated/stale items
2. Read the exported TOML, fill in `note` fields with concise descriptions
3. Run `annotate import` to merge annotations back
4. Re-run `generate` to update the index with new annotations

## Visualization

The relationships.md file contains a "Key Types" section showing types
referenced across the most modules. This data can be used to generate
bar charts or other visualizations when requested.
