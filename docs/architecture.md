# Architecture

Draxl is organized as a small Rust workspace rather than a single crate. The
split is intentionally along semantic boundaries so the core data model, parser,
printer, validator, lowering, patching, and CLI can evolve independently.

The current implementation is the Rust profile over `.rs.dx` files, but the
core source model is intended to support multiple language profiles over time.

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
          | printer   |  | draxl-rust   |  | patch ops  |
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
- parsing the bootstrap Rust profile
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

Stable formatting is one of the core repository claims, so canonicalization and
rendering stay separate.

### `draxl-rust`

This crate owns Rust-profile support. Today that surface is primarily lowering:
it lowers validated Draxl into ordinary Rust source for the currently supported
subset and strips Draxl metadata while preserving the semantic shape
represented by the AST.

### `draxl-patch`

The patch crate applies structured edit operators over the AST:

- parse and resolve canonical textual patch streams
- insert into ranked slots
- put into single-child slots
- replace node bodies while preserving outer identity
- delete and move removable nodes
- attach and detach docs/comments
- set and clear scalar fields

The patch model is deliberately semantic, slot-aware, and attachment-aware. It
provides a structured tree edit layer over the AST.

### `draxl-cli`

The CLI is thin by design. It exercises the public `draxl` facade for the main
workflows exposed to users:

- `parse`
- `fmt`
- `dump-json`
- `validate`
- `lower-rust`
- `patch`

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

### The root crate is the API boundary

The root `draxl` crate is the intended integration surface for Rust callers.
The CLI exercises that shared surface for command-line workflows.
