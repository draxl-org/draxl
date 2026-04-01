# Draxl

Draxl is an agent-native source annotation layer for deterministic, high-volume
concurrent code editing.

It makes syntax identity and ordering explicit in the source itself.

Instead of patching text spans, code edits are semantic operations over
stable node IDs:

```text
insert @f1.body[ah]: @s3 let @p3 z = @e4 (@e6 y + @l2 1);

replace @e3: @e5 z
```

## Example Draxl annotations

Draxl annotation `.rs.dx` files lower deterministically to Rust. The annotation
looks like this:

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

Lowered to Rust:

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

How to read the markers:

```text
@id[rank]->anchor
```

- `@id` gives the next supported node a stable identity
- `[rank]` orders siblings inside ranked slots
- `->anchor` explicitly attaches detached docs or comments to an existing sibling id

Doc and line comments attach implicitly to the next semantic sibling when an
explicit anchor is absent.

## What This Unlocks

Draxl is more than a merge-friendly source format. Stable node IDs, ranks,
anchors, and structured annotations create room for higher-level program control
that ordinary text files do not represent cleanly.

- **Node-level ownership and policy.** Functions, types, fields, match arms, and
  statements can carry explicit owners, required reviewers, stability levels, or
  security policy tags. Those rules stay attached to the syntax they govern
  instead of living at file scope or drifting with line edits and refactors.

- **Durable review, provenance, and audit state.** Approvals, benchmark results,
  security findings, and agent provenance can attach to specific nodes and
  survive surrounding changes that preserve identity. Tools can then invalidate
  evidence precisely when the underlying semantic target changes.

- **Machine-readable contracts and capability summaries.** Nodes can carry
  structured annotations for effects, resource access, unsafe boundaries, API
  guarantees, or other higher-level constraints. That gives agents and tooling a
  durable substrate for reasoning that is stronger than comments and cheaper
  than re-deriving intent from raw source.

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

Draxl distinguishes between hard conflicts, validation/build failures, and
semantic conflicts. For the layering model and current terminology, see
[docs/merge/conflicts.md](docs/merge/conflicts.md) and
[docs/merge/semantic-conflicts.md](docs/merge/semantic-conflicts.md).

## Semantic patching

Draxl treats semantic patch operators as first-class infrastructure for
agent-native editing, not as a convenience wrapper around text replacement.

A text diff says "these bytes changed here." A Draxl patch says "replace node
`@e2`", "insert into `@f1.body[ah]`", or "attach `@d2` to `@f1`." That is a
better execution format for replay, merge, and audit because the patch names
the semantic target directly instead of depending on nearby lines to rediscover
intent.

Text diffs are good for human review, but they are a weak machine interface for
structural edits:

- nearby inserts, formatting, and unrelated rewrites can invalidate context or
  create false conflicts
- ordered inserts, moves, and comment attachment have to be reconstructed from
  text layout
- replay across branch stacks and long-lived forks depends on matching byte
  neighborhoods, not stable program identity

Draxl patch ops target the program tree itself:

- replace a node body while preserving outer identity
- insert into a ranked slot under a parent
- put a new occupant into a single-child slot
- move or delete a specific node
- attach docs and comments explicitly
- set or clear scalar fields such as names, operators, and semicolon state

Same change, two representations:

Text diff:

```diff
@@
-    y
+    let z = (y + 1);
+    z
```

Semantic ops:

```text
insert @f1.body[ah]: @s3 let @p3 z = @e4 (@e6 y + @l2 1);

replace @e3: @e5 z
```

The text diff relies on the exact surrounding lines. The semantic ops say what
changed: insert a new statement into `@f1.body` and replace the expression node
`@e3`. As long as those ids survive, the edits remain targetable through
formatting, nearby inserts, and many refactors.

The structured Rust API and `draxl patch` both execute this textual patch
surface directly.

Patch docs now live under [docs/patching/README.md](docs/patching/README.md).

The same model can also express edits that are awkward in unified diffs, such
as `attach @d2 -> @f1`, `move @s2 -> @f1.body[ah]`, or `set @f1.name = run`.

The current path-op subset is intentionally narrow. It supports scalar fields
such as `@f1.name`, `@d1.text`, `@e7.op`, and `@s2.semi`, not arbitrary source
text replacement.

## Concurrent edit example

Starting block:

```text
@s1[a] @e1 fetch();
@s2[am] @e2 log();
@s3[b] @e3 validate();
```

If two agents express their edits as text diffs, both changes land in the same
textual neighborhood:

Agent A:

```diff
@@
 fetch();
+trace();
 log();
 validate();
```

Agent B:

```diff
@@
 fetch();
-log();
+audit();
 validate();
```

Those diffs both depend on the same hunk around `log();`, so overlap is likely
even though one change is an insertion and the other is a replacement.

Draxl keeps the operations separate:

Agent A:

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

The edits compose cleanly because they target different semantic regions: one
addresses the ranked body slot and the other replaces node `@e2`.

Not every merge-relevant overlap is a hard conflict. Draxl also tracks semantic
conflicts: cases where the patch streams remain structurally mergeable but the
combined result should still be reviewed. See
[docs/merge/semantic-conflicts.md](docs/merge/semantic-conflicts.md).

## What works today

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
- checking hard conflicts and the first semantic conflict class over pairs of
  patch streams through `draxl-merge` and the root facade
- serving workspace-scoped Draxl edit tools over stdio MCP and generating
  workspace-local Codex config through `draxl-agent` and `draxl-cli`

## Architecture

The current workspace is intentionally split by responsibility:

- `draxl`: public facade over the workspace crates
- `draxl-ast`: typed AST and metadata model
- `draxl-parser`: lexer and parser for the Draxl surface syntax
- `draxl-validate`: structural validation for ids, ranks, and anchors
- `draxl-printer`: canonicalization and source printing
- `draxl-rust`: Rust-profile support, currently centered on lowering validated
  Draxl to ordinary Rust
- `draxl-patch`: structured patch operators over ids, profile-defined slots,
  attachments, and scalar paths
- `draxl-merge`: merge analysis over patch streams, including hard conflicts
  and initial semantic conflict rules
- `draxl-agent`: workspace-scoped agent edit backend and stdio MCP server
- `draxl-cli`: command-line entry point and MCP/Codex setup wrapper

```text
source text
  -> draxl-parser
  -> draxl-validate
  -> draxl-printer / draxl-rust / draxl-patch / draxl-merge
  -> draxl-agent
  -> draxl facade and draxl CLI
```

The core model is profile-agnostic. This repository currently implements the
Rust profile and lowers validated `.rs.dx` input to Rust source.

More detail:

- [docs/architecture.md](docs/architecture.md)
- [docs/syntax.md](docs/syntax.md)
- [docs/patching/README.md](docs/patching/README.md)
- [docs/merge/README.md](docs/merge/README.md)

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

## Examples

The `examples/` directory is a guided tour of the current Rust profile:

- `examples/01_add.rs.dx` is the smallest end-to-end example: stable ids,
  ranked params and statements, implicit doc/comment attachment, and lowering
  to ordinary Rust
- `examples/02_shapes.rs.dx` shows item-level ordering over a `struct` and an
  `enum`, including ranked fields and variants
- `examples/03_match.rs.dx` exercises `match` expressions, ranked arms, guard
  expressions, binary `<`, and unary minus
- `examples/04_ranks.rs.dx` isolates lexicographic ranks such as `a`, `am`,
  and `b` to show how deterministic insertion order works inside a block
- `examples/05_use_tree_group.rs.dx` covers grouped `use` trees, `self` and
  glob imports, qualified paths, and a multi-parameter function

### Try it

```bash
cargo run -p draxl-cli -- parse examples/01_add.rs.dx
cargo run -p draxl-cli -- fmt examples/01_add.rs.dx
cargo run -p draxl-cli -- dump-json examples/01_add.rs.dx
cargo run -p draxl-cli -- validate examples/01_add.rs.dx
cargo run -p draxl-cli -- lower-rust examples/01_add.rs.dx
cargo run -p draxl-cli -- conflicts <file> <left-patch-file> <right-patch-file>
cargo run -p draxl-cli -- mcp setup --client codex --root .
cargo run -p draxl-cli -- mcp serve --root .
```

## License

Licensed under `MIT OR Apache-2.0`.

- [LICENSE-MIT](LICENSE-MIT)
- [LICENSE-APACHE](LICENSE-APACHE)
