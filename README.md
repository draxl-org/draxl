# Draxl

Draxl is an agent-native source language for high-volume concurrent
deterministic code editing.

It makes syntax identity, ordering, and attachment explicit in the source
itself, so tools can apply semantic operators over stable nodes instead of
patching text spans.

This repository contains the first Draxl language profile, which targets Rust
through `.rs.dx` files and deterministic lowering to Rust.

Under the hood, Draxl is AST-native: the source carries the structure,
identity, and attachment data that replay, merge, audit, and lowering
workflows need.

```text
agent edits -> semantic ops over stable ids -> validated Draxl -> canonical .rs.dx -> Rust
```

The `.rs.dx` extension is intentional. `.dx` names the Draxl layer and `.rs`
names the current language profile. That keeps the profile-specific surface
distinct from the higher-level source model and leaves room for future profiles
such as `.go.dx`, `.ts.dx`, and `.py.dx`.

More detail:

- [docs/architecture.md](docs/architecture.md)
- [docs/syntax.md](docs/syntax.md)
- [docs/patching.md](docs/patching.md)
- [docs/patch-op-syntax.md](docs/patch-op-syntax.md)

## Why now

Draxl is a bet on a near-future software workflow.

As inference gets cheaper, code generation becomes abundant and integration
becomes the bottleneck. Repositories will need to absorb far more
agent-produced edits, many more concurrent branches and PRs, and long-lived
forks maintained for specific users, products, and deployments.

Line-based source and text diffs are a poor fit for that environment. Byte
ranges create false conflicts, rebase churn, and weak replayability because
syntax identity is positional and tooling has to reconstruct structure from
surrounding text.

Draxl makes syntax identity explicit in the source itself. Tools can patch,
replay, merge, and audit code semantically by addressing stable node IDs and
ranked slots instead of guessing intent from lines and spans.

## Why Draxl

- stable node ids let tools target syntax directly instead of guessing by line
  and column
- ranks make ordered inserts explicit, so concurrent edits do not depend on
  textual history
- anchors make detached docs and comments attach deterministically across
  replay and merge
- canonical printing keeps human-readable source and machine output stable
- the current Rust profile preserves compatibility with the existing Rust
  toolchain through deterministic lowering

| Concern                | Text diffs                | Draxl                        |
|------------------------|---------------------------|------------------------------|
| Edit target            | byte ranges, lines, spans | stable node ids              |
| Ordered insert         | textual position          | ranked slot under a parent   |
| Comment/doc attachment | proximity heuristics      | explicit anchors             |
| Replay and audit       | surrounding text          | semantic ops over node ids   |
| Branches and forks     | repeated rebase repair    | semantic replay by identity  |
| Merge conflicts        | overlapping text          | overlapping semantic regions |

## Future capabilities

Draxl is more than a merge-friendly source format. Stable node IDs, ranks,
anchors, and structured annotations create room for higher-level program control
that ordinary text files do not represent cleanly.

- **Node-level ownership and policy.** Functions, types, fields, match arms, and
  statements can carry explicit owners, required reviewers, stability levels, or
  security policy tags. Those rules stay attached to the syntax they govern
  instead file level scope or drifting with line edits and refactors.

- **Durable review, provenance, and audit state.** Approvals, benchmark results,
  security findings, and agent provenance can attach to specific nodes and
  survive surrounding changes that preserve identity. Tools can then invalidate
  evidence precisely when the underlying semantic target changes.

- **Machine-readable contracts and capability summaries.** Nodes can carry
  structured annotations for effects, resource access, unsafe boundaries, API
  guarantees, or other higher-level constraints. That gives agents and tooling a
  durable substrate for reasoning that is stronger than comments and cheaper
  than re-deriving intent from raw source.

## Architecture

The current workspace is intentionally split by responsibility:

- `draxl`: public facade over the workspace crates
- `draxl-ast`: typed AST and metadata model
- `draxl-parser`: lexer and parser for the Draxl surface syntax
- `draxl-validate`: structural validation for ids, ranks, and anchors
- `draxl-printer`: canonicalization and source printing
- `draxl-lower-rust`: lowering from validated Draxl to ordinary Rust
- `draxl-patch`: structured patch operators over ids, profile-defined slots,
  attachments, and scalar paths
- `draxl-cli`: command-line entry point

```text
source text
  -> draxl-parser
  -> draxl-validate
  -> draxl-printer / draxl-lower-rust / draxl-patch
  -> draxl facade and draxl CLI
```

The core model is profile-agnostic. This repository currently implements the
Rust profile and lowers validated `.rs.dx` input to Rust source.

## Semantic patching

Draxl treats semantic patch operators as first-class infrastructure for
agent-native editing, not as a convenience wrapper around text replacement.

Instead of rewriting byte ranges, a tool addresses stable node IDs and ranked
slots, single-child slots, attachment relations, and schema-defined scalar
paths. An edit can replace a node body, insert into a ranked slot, put a new
occupant into a single-child slot, move a node, delete a node, attach trivia,
or update scalar fields.

That makes patches precise enough to replay across branch stacks and long-lived
forks, merge cleanly when they touch different semantic regions, and audit at
the level of the program tree.

Today the executor exposes that model through the structured Rust API. The
canonical textual notation used in the docs is not parsed yet.

The current path-op subset is intentionally narrow. It supports scalar fields
such as `@f1.name`, `@d1.text`, `@e7.op`, and `@s2.semi`, not arbitrary source
text replacement.

## Example Draxl source

```text
@m1 mod demo {
  @d1 /// Add one to x.
  @f1[a] fn add_one(@p1[a] x: @t1 i64) -> @t2 i64 {
    @c1 // Cache the intermediate value.
    @s1[a] let @p2 y = @e1 (@e2 x + @l1 1);
    @s2[b] @e3 y
  }
}
```

The metadata prefix stays compact:

```text
@id[rank]->anchor
```

- `@id` gives the next supported node a stable identity
- `[rank]` orders siblings inside ranked slots
- `->anchor` attaches detached docs or comments to an existing sibling id

Doc and line comments attach implicitly to the next semantic sibling when an
explicit anchor is absent.

## Concurrent edit example

Canonical patch notation for future tooling (not yet executable):

```text
replace @e2: (@e2 x * @l2 2)

insert @f1.body[b]: @s3 let @p3 z = @e4 (y + @l3 1);

attach @d2 -> @f1

set @f1.name = add_one_fast

clear @d1.text
```

Starting block:

```text
@s1[a] @e1 fetch();
@s2[am] @e2 log();
@s3[b] @e3 validate();
```

Agent A inserts a statement into `@f1.body` with rank `ah`:

```text
insert @f1.body[ah]: @s4 @e4 trace();
```

Agent B rewrites expression `@e2`:

```text
replace @e2: @e2 audit()
```

Merged result:

```text
@s1[a] @e1 fetch();
@s4[ah] @e4 trace();
@s2[am] @e2 audit();
@s3[b] @e3 validate();
```

The edits compose cleanly because they target different semantic nodes and
slots.

## Lowered Rust

```rust
mod demo {
    /// Add one to x.
    fn add_one(x: i64) -> i64 {
        // Cache the intermediate value.
        let y = (x + 1);
        y
    }
}
```

## Current status

Draxl currently implements the Rust profile through `.rs.dx` files
with parsing, validation, canonical formatting, JSON dumping, Rust lowering,
and bootstrap semantic patch application.

The current milestone supports:

- parsing the bootstrap Rust profile into a typed Draxl IR
- validating ids, ranks, anchors, sibling attachment, and ranked-slot
  uniqueness
- printing canonical Draxl source with deterministic ordering
- re-parsing canonical output while preserving semantics
- dumping deterministic JSON for the IR
- lowering the current profile to ordinary Rust
- applying semantic patch ops over ids, schema-defined slots, attachments, and
  the current scalar path subset: names, trivia text, operators, and statement
  semicolon state

## Try it

```bash
cargo run -p draxl-cli -- parse examples/01_add.rs.dx
cargo run -p draxl-cli -- fmt examples/01_add.rs.dx
cargo run -p draxl-cli -- dump-json examples/01_add.rs.dx
cargo run -p draxl-cli -- validate examples/01_add.rs.dx
cargo run -p draxl-cli -- lower-rust examples/01_add.rs.dx
```

## Library crate

The root crate is `draxl`: a thin public facade over the parser, validator,
printer, lowering, and patch APIs.

```rust
use draxl::{format_source, lower_rust_source, parse_and_validate};

let file = parse_and_validate("@m1 mod demo {}") ?;
let formatted = format_source("@m1 mod demo {}") ?;
let lowered = lower_rust_source(
"@m1 mod demo { @f1[a] fn run() { @s1[a] @e1 work(); } }",
) ?;
```

## Example corpus

- `examples/01_add.rs.dx`
- `examples/02_shapes.rs.dx`
- `examples/03_match.rs.dx`
- `examples/04_ranks.rs.dx`
- `examples/05_use_tree_group.rs.dx`

## Roadmap

- widen the current Rust profile beyond the bootstrap examples
- extend the patch model from insert/replace/delete to richer structural ops
- add additional language profiles without changing the core identity, rank,
  anchor, and patch model
- harden merge-friendly workflows around stable ids, ranks, and anchors

## License

Licensed under `MIT OR Apache-2.0`.

- [LICENSE-MIT](LICENSE-MIT)
- [LICENSE-APACHE](LICENSE-APACHE)
