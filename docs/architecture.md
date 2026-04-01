# Architecture

Draxl is organized as a small Rust workspace rather than a single crate. The
split is intentionally along semantic boundaries so the shared data model,
validation, patch/merge mechanics, language adapters, and CLI can evolve
independently.

The current implementation only supports the Rust lower language over `.rs.dx`
files, but the workspace now models that support as an explicit
`LowerLanguage` dispatch boundary rather than as an implicit global default.

## Data flow

```text
source .rs.dx
     |
     v
+---------------------------+
| draxl-parser              |
| dispatch by LowerLanguage |
+---------------------------+
     |
     v
+------------------+
| draxl-rust parse |
+------------------+
     |
     v
+------------------+
| draxl-ast::File  |
+------------------+
     |
     v
+------------------+
| draxl-validate   |
+------------------+
  |        |        |         |
  |        |        |         |
  v        v        v         v
printer  lowering  patch    merge
dispatch adapter   dispatch dispatch
  |        |        |         |
  v        v        v         v
draxl-   draxl-   draxl-    draxl-
rust     rust     rust      rust
render            schema    semantics
  \        |        |         /
   \       |        |        /
    +----------------------+
    |    draxl facade      |
    +----------------------+
               |
               v
         draxl CLI / users
```

## Crates

### `draxl`

The root crate is the public facade. It re-exports the internal crates and
offers the common workflows:

- parse
- validate
- parse and validate
- format
- dump JSON
- lower to Rust
- apply structured patches

The facade exists so downstream users do not need to wire the internal crates
together manually.

### `draxl-ast`

The AST crate defines the typed IR and shared metadata:

- stable node ids
- optional rank and anchor metadata
- slot names for ordered children
- typed items, statements, expressions, patterns, and types
- the `LowerLanguage` dispatch key used by higher layers
- canonical ordering of the shared tree for stable comparison and printing

This crate has no parser, validation, or rendering policy. It is the shared
data model for the rest of the workspace.

### `draxl-parser`

The parser crate is now a thin dispatch facade:

- accepts an explicit `LowerLanguage`
- routes whole-file and fragment parsing to the selected adapter
- keeps Rust-default compatibility wrappers for existing callers

The actual Rust parsing implementation lives in `draxl-rust`.

### `draxl-validate`

The validator owns file-level semantic checks:

- duplicate ids
- missing ranks in ordered slots
- duplicate ranks in a slot
- invalid anchors
- detached comments or docs

Validation is intentionally separate from parsing so the parser can stay narrow
and syntax-focused.

### `draxl-printer`

The printer crate is also a dispatch facade:

- accepts an explicit `LowerLanguage`
- routes rendering to the selected adapter
- re-exports shared canonicalization from `draxl-ast`

Stable formatting is one of the core repository claims, so canonicalization and
rendering stay separate.

### `draxl-rust`

This crate owns the Rust language adapter:

- parse `.rs.dx` files and fragments
- render canonical `.rs.dx`
- lower to ordinary Rust
- import ordinary Rust into the Draxl IR
- define Rust patch schema and slot/path rules
- define Rust semantic merge analysis

### `draxl-patch`

The patch crate applies structured edit operators over the AST:

- parse and resolve canonical textual patch streams
- dispatch fragment parsing and schema checks by `LowerLanguage`
- insert into ranked slots
- put into single-child slots
- replace node bodies while preserving outer identity
- delete and move removable nodes
- attach and detach docs/comments
- set and clear scalar fields

The patch model is deliberately semantic, slot-aware, and attachment-aware. It
provides a structured tree edit layer over the AST while keeping
Rust-default compatibility wrappers for callers that do not pass a language.

### `draxl-merge`

The merge crate owns conflict detection over patch streams:

- hard-conflict detection over structured patch ops
- replay-based convergence checks
- semantic conflict extraction dispatched by `LowerLanguage`
- structured conflict reports for humans and agents

### `draxl-agent`

The agent crate owns workspace-scoped semantic editing and the stdio MCP server:

- workspace root restriction for `.rs.dx` edits
- file inspection, node lookup, and fingerprinting
- high-level edit operations such as `replace_node`, `insert_after_stmt`, and
  `set_path`
- raw patch-text application and conflict checks
- stdio MCP tool serving over that shared backend

### `draxl-cli`

The CLI is thin by design. It exercises the public `draxl` facade for the core
language workflows and `draxl-agent` for the agent-facing MCP workflow:

- `parse`
- `fmt`
- `dump-json`
- `validate`
- `lower`
- `lower-rust`
- `patch`
- `conflicts`
- `mcp serve`
- `mcp setup --client codex`

For commands that operate on Draxl source files, the CLI infers `LowerLanguage`
from the source file extension such as `.rs.dx`. The Rust library API keeps the
language explicit instead of relying on file names.

## Design choices

### Stable ids live in source

Draxl carries node identity directly in source through metadata prefixes.

### Ordered children are explicit

Lists that matter for concurrency use ranks rather than textual order alone.
This lets tools describe insertions without depending on the exact byte layout
of nearby code.

### Validation is a first-class phase

The parse phase accepts the supported syntax. The validate phase decides whether
the resulting tree satisfies the stronger semantic invariants that make
canonical printing and patching predictable.

### Language dispatch stays explicit at the API boundary

Library callers can choose a `LowerLanguage` directly. The CLI performs file
extension detection and then calls those explicit APIs. Compatibility wrappers
still default to Rust so existing callers keep working while the adapter split
stays visible in the architecture.

### The root crate is the API boundary

The root `draxl` crate is the intended integration surface for Rust callers.
The CLI exercises that shared surface for command-line workflows, while
`draxl-agent` packages the workspace-scoped agent/MCP behavior that does not
belong in the public Rust facade.
