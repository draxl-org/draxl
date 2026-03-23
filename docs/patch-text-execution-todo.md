# Patch Text Execution TODO

This document originally turned the canonical patch notation into an
implementation plan. The core textual patch surface is now executable through
the Rust API and CLI for the current modeled Rust profile.

The remaining notes here are best read as follow-up cleanup and extension work,
not as a statement that patch-text execution is still missing.

The goal is not to bypass the existing structured patch model. The goal is:

```text
textual patch stream -> surface patch AST -> schema + AST resolution -> PatchOp -> executor
```

That keeps the public patch language schema-backed while preserving the current
typed executor as the resolved backend.

## Goal

Keep the canonical textual patch notation in
[patch-op-syntax.md](patch-op-syntax.md) executable and schema-backed for the
current Rust profile.

At the end of this work, users should be able to:

- parse one or more textual patch ops
- resolve them against a current Draxl file
- apply them through the existing executor
- use the same surface through the Rust facade and CLI

## Non-goals

- do not collapse the patch surface into raw Rust struct field names
- do not parse text directly into the final `PatchOp` enum without resolution
- do not reintroduce kind-specific verbs such as `replace_expr`
- do not treat id spelling as the source of truth for node kind
- do not make the patch parser a second copy of AST mutation logic

## Required semantics

The implementation must preserve the current semantic direction:

- ids are semantic locators
- kind inference comes from schema plus AST lookup
- `replace` preserves the target node shell
- `put` sets single-child slot occupancy and may change outer identity
- attachments are identity-bound, not slot-bound
- `move` carries attachment closure
- `attach` and `detach` are first-class relation ops
- path updates use profile-defined field names and value kinds

## Workstreams

### 1. Fix the textual spec before coding

The canonical syntax doc still has gaps and stale examples that should be
resolved before parser implementation.

TODO:

- update `replace` examples to preserve outer identity
  Example: `replace @e2: (@e2 x * @l2 2)` instead of `replace @e2: (@e9 ...)`
- define `value` grammar explicitly
  Supported current subset:
  - identifier values for names and operator tags
  - quoted strings for `text`
  - integers where allowed
  - `true` / `false` for booleans
- define patch-stream structure
  - one op per stream entry
  - allow blank lines between ops
  - do not rely on line splitting for fragment boundaries
- define fragment termination rules for multiline fragments
  - balanced braces, parens, and brackets must be respected
  - future parser should consume a full fragment, not a line suffix
- define exact surface names for the currently supported scalar path subset
  - `name`
  - `text`
  - `op`
  - `semi`
- define textual examples only within the currently modeled Rust profile

### 2. Add a textual patch frontend in `draxl-patch`

The textual patch language should live with the patch model, not with the main
`.rs.dx` source parser.

Reason:

- `draxl-parser` owns Draxl source syntax
- `draxl-patch` owns patch syntax and patch semantics
- the patch frontend needs to resolve against patch schema and current AST state

TODO:

- add a patch lexer/parser module under `crates/draxl-patch/src/`
- introduce a patch parse error type with spans and line/column reporting
- parse the textual surface into an unresolved surface representation, not
  directly into `PatchOp`

Suggested surface types:

- `SurfacePatchOp`
- `SurfaceDest`
- `SurfaceSlotRef`
- `SurfacePath`
- `SurfaceValue`
- `SurfaceFragment`

The surface fragment should initially retain either:

- raw fragment source text plus span, or
- a parsed fragment node plus fragment kind metadata

### 3. Expose fragment parsers from `draxl-parser`

The patch frontend needs to parse Draxl fragments that appear after `:`.

The current parser already has internal support for parsing:

- items
- statements
- expressions
- types
- patterns
- match arms

But those entry points are not public today.

TODO:

- expose fragment parsing helpers from `crates/draxl-parser`
- keep them narrow and explicit
- avoid duplicating expression or statement parsing in `draxl-patch`

Likely public helpers:

- `parse_item_fragment`
- `parse_stmt_fragment`
- `parse_expr_fragment`
- `parse_type_fragment`
- `parse_pattern_fragment`
- `parse_match_arm_fragment`

The patch frontend can then choose the right helper during resolution.

### 4. Introduce a profile schema layer for patch resolution

Executable patch text needs a profile schema layer because slot/path knowledge
is currently spread across executor modules. It needs one source of truth for:
- public slot names
- ranked vs single-child slots
- allowed fragment kind for each slot
- public path names
- value kind for each path
- attachable relations and constraints

TODO:

- add a bootstrap Rust patch schema module under `crates/draxl-patch`
- centralize slot/path metadata currently hardcoded in:
  - `insert.rs`
  - `put.rs`
  - `move.rs`
  - `delete.rs`
  - `set_clear.rs`
  - attachment checks
- keep public names stable even if Rust struct field names change

Suggested schema questions each entry must answer:

- what owners can expose this slot/path?
- what node kind is expected?
- is the slot ranked or single-child?
- what fragment kind is legal here?
- what value kind is legal here?
- can this relation carry attachments?

### 5. Add a resolver from surface ops to typed `PatchOp`

The textual parser alone is insufficient. Several ops need AST lookup and schema
resolution before they can become executable.

Examples:

- `replace @e2: ...` needs target lookup to determine fragment kind
- `put @f1.ret: ...` needs slot lookup to determine fragment kind
- `set @s2.semi = true` needs path lookup to determine value kind

TODO:

- add a resolver module in `draxl-patch`
- resolve node refs against the current file
- resolve slot refs through the schema
- resolve path value types through the schema
- parse fragment text using the correct fragment parser after resolution
- lower the resolved result into existing typed `PatchOp`

The resolver should be the layer that enforces:

- kind inference from schema plus AST lookup
- shell-preserving `replace`
- slot-oriented `put`
- valid attach/detach targets
- legal move destinations for attachment closure

### 6. Reuse the existing executor as the backend

Do not fork executor logic into the textual frontend.

TODO:

- keep `apply_op` and `apply_ops` as the execution backend
- make textual patch application lower into ordinary `PatchOp`
- keep validation and canonicalization behavior unchanged after application

Target flow:

```text
parse patch text -> resolve -> apply existing PatchOp -> validate -> print/lower/test
```

### 7. Expand the public API

The root `draxl` facade should expose textual patch support, not just the
low-level patch crate.

TODO:

- add a facade method to parse textual patch ops
- add a facade method to apply textual patch ops to a parsed file
- decide whether facade errors stay split into parse-vs-apply or gain a unified
  patch-text error wrapper

Possible API shape:

- `parse_patch_ops(text: &str) -> Result<Vec<SurfacePatchOp>, ...>`
- `resolve_patch_ops(file: &ast::File, text: &str) -> Result<Vec<PatchOp>, ...>`
- `apply_patch_text(file: &mut ast::File, text: &str) -> Result<(), ...>`

### 8. Add CLI support

The CLI should be able to execute a patch stream against a `.rs.dx` file.

TODO:

- add a `patch` command to `draxl-cli`
- choose an input shape

Recommended first shape:

```text
draxl patch [--in-place] <file> <patch-file>
```

Behavior:

- read the Draxl source file
- parse and validate it
- read the patch text file
- parse and resolve patch ops
- apply them in order
- validate the result
- print the canonical result or write it in place

Later additions can include stdin support, but the file-based path is simpler
for the first implementation and matches current CLI patterns.

### 9. Add tests at every layer

This work needs more than executor tests.

TODO:

- patch lexer/parser tests
  - every verb
  - valid and invalid refs
  - value parsing
  - multiline fragments
- resolver tests
  - correct slot/path typing
  - fragment-kind inference
  - invalid destination and path errors
- executor integration tests using textual patch input
  - exact README examples
  - `replace`, `insert`, `put`, `move`, `attach`, `detach`, `set`, `clear`
- CLI e2e tests for `draxl patch`
- negative tests for:
  - stale `replace` outer ids
  - invalid slot names
  - invalid path names
  - wrong value kinds
  - nonexistent node refs
  - illegal attachment retargeting
  - moves into invalid attachment contexts

### 10. Update docs after the implementation lands

Once the textual frontend is real, the docs need to stop calling it future
tooling.

TODO:

- update `README.md`
- update `docs/patching.md`
- update `docs/patch-op-syntax.md`
- update `docs/architecture.md`
- add CLI usage examples for textual patch execution
- align all examples with shell-preserving `replace`

## Recommended implementation order

1. Fix the spec gaps in `patch-op-syntax.md`.
2. Add patch parse error types and surface AST types in `draxl-patch`.
3. Expose fragment-level parsing helpers from `draxl-parser`.
4. Add the bootstrap Rust patch schema layer.
5. Implement parser for `insert` / `delete` / `attach` / `detach` / `clear`.
6. Implement resolver for `set`, `move`, `put`, and `replace`.
7. Add facade APIs.
8. Add CLI `patch` command.
9. Add integration and CLI e2e coverage.
10. Update docs to present textual patch execution as implemented.

This order keeps the hard part in the middle: schema-backed resolution.

## Concrete first milestone

A good first milestone is:

- parse a patch file containing `insert`, `delete`, `attach`, `detach`, `set`,
  and `clear`
- resolve and apply them against a Draxl file
- support `replace` and `put` only after the schema resolver is in place

That still delivers real executable patch text early without faking the hard
semantics.

## Open questions to settle during implementation

- Should patch parsing errors live in `draxl-patch` or share infrastructure with
  `draxl-parser`?
- Should the surface parser own tokenization, or should a reusable token layer
  be extracted?
- Should a patch stream accept comments?
- Should the CLI support stdin patch streams in v1, or only patch files?
- Should we expose unresolved surface ops publicly, or keep them crate-private
  and expose only resolved `PatchOp` values?

## Done criteria

This work is complete when:

- the canonical patch text is executable through Rust API and CLI
- textual ops lower into ordinary typed `PatchOp`
- the resolver is schema-backed rather than field-name-backed
- the executor remains the single mutation backend
- docs no longer describe textual patch parsing as future work
- repo tests cover parser, resolver, executor integration, and CLI behavior
