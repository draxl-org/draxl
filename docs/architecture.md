# Architecture

Draxl is organized as a small Rust workspace rather than a single crate. The
split is intentionally along semantic boundaries so the core data model, parser,
printer, validator, lowering, patching, and CLI can evolve independently.

## Data flow

```text
                +----------------+
source .rs.dx -->| draxl-parser   |
                +----------------+
                         |
                         v
                +----------------+
                | draxl-ast::File|
                +----------------+
                         |
                         v
                +----------------+
                | draxl-validate |
                +----------------+
                  |      |      |
                  |      |      |
                  v      v      v
          +-----------+  +--------------+  +------------+
          | printer   |  | lower-rust   |  | patch ops  |
          +-----------+  +--------------+  +------------+
                  \         |                 /
                   \        |                /
                    +------------------------+
                    |      draxl facade      |
                    +------------------------+
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

This crate has no parser, validation, or rendering policy. It is the shared
data model for the rest of the workspace.

### `draxl-parser`

The parser crate owns the surface syntax front end:

- lexing sigil metadata such as `@f1[a]->x2`
- parsing the Rust-shaped bootstrap subset
- attaching metadata to typed AST nodes
- producing structured parse errors with spans and line/column locations

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

The printer has two jobs:

- canonicalize ordered children and comment/doc placement
- render the AST back into canonical Draxl source

That split is important because stable formatting is one of the core repository
claims.

### `draxl-lower-rust`

This crate lowers validated Draxl into ordinary Rust source for the currently
supported subset. It strips Draxl metadata while preserving the semantic shape
represented by the AST.

### `draxl-patch`

The patch crate applies structured edit operators over the AST:

- insert into a slot under a parent id
- replace a ranked child by id
- delete a ranked child by id

The patch model is deliberately typed and slot-aware. It is not a generic text
rewrite layer.

### `draxl-cli`

The CLI is thin by design. It exercises the public `draxl` facade for the main
workflows exposed to users:

- `parse`
- `fmt`
- `dump-json`
- `validate`
- `lower-rust`

## Design choices

### Stable ids live in source

Draxl is not trying to infer identity after the fact. The source already
contains node identity through metadata prefixes.

### Ordered children are explicit

Lists that matter for concurrency use ranks rather than textual order alone.
This lets tools describe insertions without depending on the exact byte layout
of nearby code.

### Validation is a first-class phase

The parse phase accepts the supported syntax. The validate phase decides whether
the resulting tree satisfies the stronger semantic invariants that make
canonical printing and patching predictable.

### The CLI is not the API boundary

The root `draxl` crate is the intended integration surface for Rust callers.
The CLI is a client of that surface, not a second copy of the same logic.
