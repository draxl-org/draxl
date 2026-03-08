# Draxl

Draxl is an agent-native source language with explicit syntax identity.

It makes syntax identity explicit with stable node IDs, ranks, anchors, and
semantic patch operators, so tools can edit the program tree directly instead
of patching text spans.

Under the hood, Draxl is AST-native: the source carries the structure,
identity, and attachment data that replay, merge, audit, and lowering
workflows need.

```text
agent edits -> semantic ops over stable ids -> validated Draxl -> canonical .rs.dx -> Rust
```

Current Draxl source files use the target-qualified `.rs.dx` extension so the
lowering target stays explicit. That leaves room for future backends to use
their own `<target>.dx` suffix.

More detail:

- [docs/architecture.md](docs/architecture.md)
- [docs/syntax.md](docs/syntax.md)
- [docs/patching.md](docs/patching.md)

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
- lowering to Rust preserves compatibility with the existing Rust toolchain

| Concern                | Text diffs                | Draxl                        |
|------------------------|---------------------------|------------------------------|
| Edit target            | byte ranges, lines, spans | stable node ids              |
| Ordered insert         | textual position          | ranked slot under a parent   |
| Comment/doc attachment | proximity heuristics      | explicit anchors             |
| Replay and audit       | surrounding text          | semantic ops over node ids   |
| Branches and forks     | repeated rebase repair    | semantic replay by identity  |
| Merge conflicts        | overlapping text          | overlapping semantic regions |

## Architecture

The current workspace is intentionally split by responsibility:

- `draxl`: public facade over the workspace crates
- `draxl-ast`: typed AST and metadata model
- `draxl-parser`: lexer and parser for the Draxl surface syntax
- `draxl-validate`: structural validation for ids, ranks, and anchors
- `draxl-printer`: canonicalization and source printing
- `draxl-lower-rust`: lowering from validated Draxl to ordinary Rust
- `draxl-patch`: structured patch operators over ids and ranked slots
- `draxl-cli`: command-line entry point

```text
source text
  -> draxl-parser
  -> draxl-validate
  -> draxl-printer / draxl-lower-rust / draxl-patch
  -> draxl facade and draxl CLI
```

## Semantic patching

Draxl treats semantic patch operators as first-class infrastructure for
agent-native editing, not as a convenience wrapper around text replacement.

Instead of rewriting byte ranges, a tool addresses stable node IDs and ranked
slots. An edit can replace an expression, insert a statement, move an item,
delete a node, or attach documentation to an explicit target.

That makes patches precise enough to replay across branch stacks and long-lived
forks, merge cleanly when they touch different semantic regions, and audit at
the level of the program tree.

The bootstrap library currently implements insert, replace, and delete over
node ids and slot ranks. The same addressing model is designed to extend to
richer structural operators.

## Example patch ops

Illustrative syntax:

```text
replace_expr @e2 with (@e9 x * @l2 2)

insert_stmt after @s1 rank=b: @s3[b] let @p3 z = @e4 (@e3 y + @l3 1);

attach_doc @d2 -> @f1
```

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

Starting block:

```text
@s1[a] @e1 fetch();
@s2[am] @e2 log();
@s3[b] @e3 validate();
```

Agent A inserts a statement after `@s1` by choosing rank `ah`:

```text
insert_stmt after @s1 rank=ah: @s4[ah] @e4 trace();
```

Agent B rewrites expression `@e2`:

```text
replace_expr @e2 with @e9 audit();
```

Merged result:

```text
@s1[a] @e1 fetch();
@s4[ah] @e4 trace();
@s2[am] @e9 audit();
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

`Draxl Source v0` currently supports a small Rust-shaped subset with parsing,
validation, canonical formatting, JSON dumping, Rust lowering, and bootstrap
semantic patch application.

The current milestone supports:

- parsing a narrow Draxl subset into a typed Rust-shaped IR
- validating ids, ranks, anchors, sibling attachment, and ranked-slot
  uniqueness
- printing canonical Draxl source with deterministic ordering
- re-parsing canonical output while preserving semantics
- dumping deterministic JSON for the IR
- lowering the supported subset to ordinary Rust
- applying bootstrap insert/replace/delete patch ops over node ids and ranked
  slots

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

let file = parse_and_validate("@m1 mod demo {}")?;
let formatted = format_source("@m1 mod demo {}")?;
let lowered = lower_rust_source(
    "@m1 mod demo { @f1[a] fn run() { @s1[a] @e1 work(); } }",
)?;
```

## Example corpus

- `examples/01_add.rs.dx`
- `examples/02_shapes.rs.dx`
- `examples/03_match.rs.dx`
- `examples/04_ranks.rs.dx`
- `examples/05_use_tree_group.rs.dx`

## Roadmap

- widen the Rust-shaped subset beyond the bootstrap examples
- extend the patch model from insert/replace/delete to richer structural ops
- harden merge-friendly workflows around stable ids, ranks, and anchors

## License

Licensed under `MIT OR Apache-2.0`.

- [LICENSE-MIT](LICENSE-MIT)
- [LICENSE-APACHE](LICENSE-APACHE)
