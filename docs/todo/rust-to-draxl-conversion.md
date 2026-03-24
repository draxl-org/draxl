# Rust To Draxl Conversion TODO

This document captures the missing reverse direction of the current Rust
profile pipeline.

Current state:

- Draxl can parse `.rs.dx`, validate it, patch it, and lower it back to
  ordinary Rust
- the workspace already has `draxl-rust` for Rust-profile support, currently
  covering validated Draxl to Rust lowering
- the public repo does not yet have a supported path from ordinary Rust source
  into Draxl source with stable ids and ranks

That gap limits the project in a few important ways:

- examples stay small and hand-authored
- semantic-conflict work is hard to exercise on realistic code
- benchmarking against real Git history is awkward
- testing realistic patch streams requires manual translation work

## Goal

Build a Rust-to-Draxl conversion path that can take ordinary Rust source in the
currently supported subset and produce valid `.rs.dx` output with stable node
ids and required ordering metadata.

That conversion should unlock:

- larger and more realistic semantic-conflict examples
- translation of real Git commits into Draxl patch streams
- corpus-building for merge and conflict benchmarks
- easier authoring of fixtures and demos from ordinary Rust code

## Default Direction

Default direction for now:

- treat Rust-to-Draxl conversion as a separate import pipeline, not as an
  extension of lowering
- target the currently supported bootstrap Rust subset first
- generate deterministic ids and ranks from AST position and structural role
- prefer a valid, stable `.rs.dx` result over preserving every detail of the
  original Rust surface layout

Reason:

- the importer should produce Draxl source that validates and round-trips
  through the existing pipeline
- the supported subset is already bounded by the current parser and lowering
  profile
- deterministic conversion is more important than perfect source fidelity in
  the first version

## Non-goals

- do not attempt full Rust language coverage in the first version
- do not preserve comments, formatting, or macro-heavy surface syntax unless it
  falls inside the supported profile
- do not block the importer on semantic-conflict work outside the current Rust
  subset
- do not require persisted merge-specific metadata beyond ordinary Draxl ids,
  ranks, and anchors

## Missing Pieces

### 1. Rust Front End

The importer needs a Rust parser front end for ordinary `.rs` files.

Initial need:

- parse the supported Rust subset into a source AST
- reject unsupported syntax clearly
- expose enough structure to assign Draxl ids and ranks deterministically

Default direction:

- use a dedicated Rust parser crate for import
- keep the front end separate from `draxl-parser`, which should remain the
  parser for Draxl syntax

Open question:

- should the first version use `syn`, `rust-analyzer` parser machinery, or a
  smaller dedicated parser for the supported subset?

### 2. Structural Mapping

The importer needs a clear mapping from ordinary Rust AST nodes into Draxl AST
nodes.

Initial need:

- items, statements, expressions, patterns, and types already supported by the
  Draxl Rust profile
- correct ownership and slot placement for ordered and single-child regions
- stable mapping rules for grouped `use` trees, ranked params, ranked block
  statements, and match arms

TODO:

- define the Rust-node to Draxl-node mapping for the supported subset
- document where the importer must invent metadata that ordinary Rust does not
  carry explicitly
- keep the mapping profile-backed rather than heuristic-only

### 3. Id And Rank Synthesis

Ordinary Rust source has no built-in Draxl ids or ranks, so the importer must
create them.

Initial need:

- deterministic node ids
- deterministic ranks for ordered slots
- deterministic attachment anchors for comments and docs if those are brought
  into scope later

Default direction:

- ids should be deterministic within one input file and stable across repeated
  imports of unchanged code
- ranks should be assigned from the original source order in ordered slots
- the first version can optimize for deterministic regeneration rather than
  cross-edit identity preservation

Open question:

- how much stability across ordinary Rust edits do we want from the first id
  assignment scheme?

### 4. Validation And Roundtrip Guarantees

The importer should feed directly into the existing Draxl validation and
lowering pipeline.

Acceptance requirements for imported output:

- the generated `.rs.dx` parses
- it validates under the current profile
- lowering it back to Rust preserves the supported semantic shape

TODO:

- run imported output through `draxl-parser` and `draxl-validate`
- add roundtrip tests from Rust input to Draxl output to lowered Rust
- define what semantic equivalence means for the supported subset

### 5. Unsupported Syntax Strategy

The importer needs a clear story for Rust code outside the supported subset.

Initial need:

- fail early with a precise unsupported-feature error
- point at the Rust construct that blocked import
- avoid partially importing files that look valid but silently lose meaning

Default direction:

- unsupported syntax should be explicit failure, not silent dropping
- the error surface should help build out the supported subset incrementally

### 6. Benchmarks And Realistic Corpora

The importer is one of the main prerequisites for serious benchmarking of
semantic conflict detection.

Desired follow-on work once import exists:

- translate real Rust files into Draxl
- translate paired Git commits into Draxl patch streams
- compare Git conflict results, compiler/build outcomes, and Draxl semantic
  conflict findings
- build a small corpus of realistic merge cases with expected classifications

TODO:

- design a benchmark harness that can consume imported Draxl fixtures
- collect a first corpus of realistic Rust commits in the supported subset
- measure detection quality and runtime across Git, build checks, and Draxl
  conflict layers

## Likely Shape

The cleanest long-term shape is probably to grow `draxl-rust` into the Rust
profile crate rather than splitting Rust support by phase.

Working default:

- extend `draxl-rust`
- parse ordinary Rust source
- map into `draxl-ast::File`
- validate
- print canonical `.rs.dx`

The root `draxl` facade and CLI can expose that importer after the mapping is
stable.

## Suggested Work Order

1. Pick the Rust parser front end for the supported subset.
2. Define the mapping table from supported Rust AST nodes to Draxl AST nodes.
3. Implement deterministic id and rank synthesis.
4. Build importer output that passes parse, validate, and lower-rust checks.
5. Add roundtrip tests on small Rust fixtures.
6. Add larger realistic fixtures and start a benchmark corpus for merge work.

## Acceptance Bar For The First Version

Before calling the first Rust-to-Draxl importer useful, it should:

- import a non-trivial Rust subset already supported by the Draxl Rust profile
- emit canonical `.rs.dx` that validates without manual editing
- lower back to Rust with the same supported semantic shape
- fail clearly on unsupported Rust constructs
- make it practical to build realistic semantic-conflict fixtures from ordinary
  Rust files and commits
