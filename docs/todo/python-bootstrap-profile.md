# Python Bootstrap Profile TODO

This document defines the first public Python profile target for Draxl.

Current state:

- Draxl publicly supports the Rust profile over `.rs.dx`
- the shared AST, parser, printer, validator, patching model, and merge
  semantics still encode Rust-specific constructs directly
- there is no `.py.dx` surface, Python lowering path, or Python import path

## Goal

Define the smallest Python profile that is worth implementing as a second
language profile.

That first profile should:

- prove that Draxl can support a non-Rust syntax without pretending to cover
  Python broadly
- cover small function-oriented modules that are useful for examples, fixtures,
  and conflict tests
- keep lowering target simple and deterministic over ordinary `.py` output
- fail clearly on unsupported syntax instead of silently dropping meaning

## Bootstrap Subset

The first Python profile should target `.py.dx` files with this modeled subset.

### Module shape

- top-level function definitions
- line comments
- ordered top-level items with explicit ranks

The first version should defer imports, classes, and decorators. Imports are
valuable, but the current shared `use` model is Rust-specific, so import syntax
belongs after the first cross-language refactor.

### Function definitions

- `def` items with a stable id on the function
- ordered parameters with stable ids and ranks
- statement bodies with stable ids and ranks

The first version should defer:

- return type annotations
- parameter annotations
- default parameter values
- positional-only and keyword-only parameters
- `*args` and `**kwargs`
- `async def`
- nested function definitions

### Statements

- assignment statements of the form `name = expr`
- `return expr`
- bare expression statements
- attached line comments

The first version should defer:

- `if`
- `for`
- `while`
- `match`
- `try`
- `with`
- `raise`
- `pass`
- destructuring assignment
- augmented assignment

Requiring non-empty bodies is acceptable in the bootstrap subset. Supporting
empty bodies cleanly would require a modeled `pass` statement.

### Expressions

- name references
- integer literals
- string literals
- grouped expressions
- call expressions
- attribute access such as `obj.attr`
- binary expressions with `+`, `-`, and `<`
- unary minus

The first version should defer:

- boolean operators
- `==`, `!=`, `<=`, `>`, `>=`
- list, tuple, dict, and set literals
- indexing and slicing
- lambdas
- comprehensions
- assignment expressions
- `await`

### Comments And Docs

- ordinary line comments lower to `#`
- explicit attachment and anchoring should keep using the existing Draxl
  metadata model

The first version should defer Python docstrings. They are meaningful runtime
expressions in Python, not pure comments, so they need a deliberate mapping.

## Default Direction

Default direction for the bootstrap profile:

- keep the existing Draxl metadata form `@id[rank]->anchor`
- keep ranks on ordered module items, parameters, and statements
- keep the subset small enough that the first implementation can exercise the
  profile boundary end to end
- prefer profile-specific parsing and lowering over stretching Rust-specific
  surface rules further

## First Shared-IR Pressure

This subset exposes a small set of shared-IR gaps that should be addressed
before any serious Python parser work begins.

### 1. Return Statement

The shared statement model needs an explicit `return` statement.

Reason:

- Python uses `return` directly
- `return` is also useful outside Python, so this is a cross-language addition
- modeling `return expr` as a bare expression statement would lose semantics

### 2. Assignment Statement

The shared statement model needs assignment separate from Rust-style `let`.

Reason:

- Python assignment introduces or rebinds names without a `let` keyword
- reusing the current `let` node would make the shared model more Rust-shaped,
  not less
- a neutral assignment form is also useful for other future profiles

### 3. Attribute Expression

The shared expression model needs attribute access separate from lexical paths.

Reason:

- `pkg.mod` style paths and `obj.attr` member access are different constructs
- Python relies on attribute access heavily even in tiny programs
- treating member access as a path would collapse value-level structure

### 4. Profile-Specific Surface Layers

Parsing and rendering need clearer profile boundaries.

Reason:

- the current parser and printer still encode Rust keywords and Rust surface
  forms directly
- Python support should add a Python profile crate rather than extend the Rust
  surface parser further

## Unsupported Syntax Strategy

Unsupported Python syntax should be explicit failure.

The bootstrap profile should:

- reject unsupported constructs with a precise error
- avoid partial imports or partial lowerings that silently change program
  meaning
- keep the boundary visible so the supported subset can expand deliberately

## Suggested Work Order

1. Land this subset definition.
2. Add shared IR for `return`, assignment, and attribute access.
3. Move parsing and rendering boundaries toward profile-specific crates.
4. Add a narrow Python profile crate that can lower the bootstrap subset to
   ordinary Python.
5. Add small `.py.dx` fixtures and roundtrip tests for parse, validate, format,
   patch, and lower.

## Acceptance Bar For The First Version

Before calling the first Python profile useful, it should:

- parse a small but non-trivial `.py.dx` subset built around function-oriented
  modules
- validate ids, ranks, anchors, and attachment behavior under the shared rules
- lower deterministically to ordinary Python for the supported subset
- fail clearly on unsupported Python syntax
- make it practical to build Python fixtures for patching and conflict work
