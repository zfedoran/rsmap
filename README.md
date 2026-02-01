# rsmap

A CLI tool that parses Rust codebases and generates multi-layered, LLM-friendly index files for efficient codebase comprehension without full-file scanning.

## What it does

Given a Rust project, `rsmap` produces four output files:

| File | Purpose | Target audience |
|------|---------|-----------------|
| `overview.md` | Crate info + module tree with descriptions | Quick orientation |
| `api-surface.md` | All item signatures (bodies stripped), grouped by module | API understanding |
| `relationships.md` | Trait impls, error chains, module deps, type hotspots | Architecture mapping |
| `index.json` | File:line lookup table for every item | Tooling / on-demand source fetch |

Plus an annotation system (`annotations.toml`) that lets you attach LLM-generated descriptions to items and track staleness across rebuilds.

## Install

```bash
cargo install --path .
```

Or build from source:

```bash
cargo build --release
```

## Usage

### Generate index

```bash
# Index the current directory
rsmap generate

# Index a specific project
rsmap generate --path /path/to/project

# Force full rebuild (ignore cache)
rsmap generate --no-cache

# Custom output directory
rsmap generate --output my-index/
```

Output goes to `.codebase-index/` by default (relative to the project path).

### Annotate items

Export unannotated items for LLM consumption:

```bash
rsmap annotate export --path /path/to/project > to_annotate.toml
```

This outputs a structured prompt with item signatures that need descriptions. Feed it to an LLM, get back filled-in TOML, then import:

```bash
rsmap annotate import annotated.toml
```

Annotations are merged into `annotations.toml` and appear inline in Layer 0 and Layer 1 outputs on the next `generate`.

## Output layers

### Layer 0 — Overview (`overview.md`)

```markdown
# Crate: my_app (bin)
Edition: 2021
Version: 0.1.0
External deps: tokio, serde, sqlx, tracing, clap

## Module Tree
- crate — Main entry point, CLI setup
  - config — CLI args, env parsing, config file loading
  - db — Database connection pool setup
    - models — Row types and query builders
  - engine — Core business logic
    - eval — Expression evaluator
```

### Layer 1 — API Surface (`api-surface.md`)

All items (pub and private), grouped by module, signatures only:

```markdown
# crate::engine::eval
<!-- file: src/engine/eval.rs -->

## Types

pub struct EvalContext<'a> {
    pub scope: &'a Scope,
    pub depth: usize,
    max_depth: usize,
}

## Functions

pub fn evaluate(expr: &Expr, ctx: &mut EvalContext) -> Result<Value, EvalError>;
fn resolve_name(name: &str, scope: &Scope) -> Option<Value>;
```

### Layer 2 — Relationships (`relationships.md`)

```markdown
## Trait Implementations
Evaluable <- Expr, LiteralExpr, BinaryExpr
Display   <- Value, EvalError, ApiError

## Error Chains
EvalError -> EngineError -> ApiError

## Module Dependencies
api          -> engine, db, config
engine::eval -> engine::plan, db::models

## Key Types (referenced from 3+ modules)
Value       — used in 8 modules
EvalContext  — used in 5 modules
```

### Layer 3 — JSON Index (`index.json`)

```json
{
  "crate::engine::eval::EvalContext": {
    "file": "src/engine/eval.rs",
    "line_start": 15,
    "line_end": 42,
    "kind": "struct",
    "visibility": "pub"
  }
}
```

## Incremental rebuilds

Files are hashed with BLAKE3. On subsequent runs, only changed files are re-parsed. All layer files are regenerated (they're cheap to write; parsing is the expensive part).

## Annotation staleness

When an item's source changes between runs:
- Its annotation is marked `stale = true`
- New items get empty annotations
- Removed items are marked `removed = true` (not deleted, for reference)

## Project structure

```
src/
  main.rs           — CLI entry (clap), subcommands
  model.rs          — Data model: CrateInfo, Module, Item, etc.
  parse.rs          — syn-based source parsing, signature extraction
  metadata.rs       — cargo_metadata integration (workspace, deps)
  resolve.rs        — Module tree building, path resolution
  layer0.rs         — Overview generator (crate/module map)
  layer1.rs         — API skeleton generator (all signatures)
  layer2.rs         — Relationship graph generator
  layer3.rs         — JSON index generator (file:line lookup)
  annotations.rs    — Annotation file management + merge
  cache.rs          — File hashing, incremental rebuild
  output.rs         — Markdown/text formatting utilities
```

## Dependencies

- **syn** — Rust source parsing (full AST)
- **cargo_metadata** — Workspace/crate structure, external deps
- **clap** — CLI
- **serde** / **serde_json** / **toml** — Serialization
- **walkdir** — Source file discovery
- **blake3** — Fast file hashing
- **anyhow** — Error handling

## License

MIT
