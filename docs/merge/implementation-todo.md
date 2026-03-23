# Merge Conflicts TODO

This document captures the next layer of merge analysis work after the initial
hard-conflict checker landed in `draxl-merge`.

Current state:

- hard conflicts have a dedicated crate: `crates/draxl-merge`
- hard conflicts already return structured explanations
- semantic conflicts exist, but only for a narrow starting set of rules
- local semantic conflicts now use semantic owners and semantic regions derived
  from the base AST on the fly

The missing work is not one big feature. It is a set of smaller foundations
that semantic conflict rules will need.

## Goal

Build a merge-analysis layer that can report:

- hard conflicts
- semantic conflicts
- rich explanations for both

The explanations should be good enough for both humans and agents:

- what conflicted
- why it conflicted
- what tree region or binding was involved
- what the next step should be

## Default Direction

Default direction for now:

- derive merge context on the fly from the base AST inside `draxl-merge`
- do not persist merge-only context in Draxl source syntax yet

Reason:

- we already have stable node ids and explicit slots
- merge analysis is currently a derived view over one base tree plus two patch
  streams
- persisting extra merge metadata in source should wait until there is a clear
  need beyond runtime indexing or profiling

This does not rule out persisted merge metadata later. It just avoids taking on
that syntax and compatibility cost too early.

## Non-goals

- do not spread merge reasoning across `draxl-validate`, `draxl-parser`, and
  the CLI
- do not persist ad hoc merge-only metadata in source before the need is clear
- do not jump straight to full compiler-grade name resolution
- do not make semantic conflict rules rely only on replay failure

## Missing Foundations

### 1. Tree Context

Semantic conflict rules need more context than exact patch targets.

At minimum we need an index derived from the base AST that can answer:

- what node kind a stable id refers to
- who its parent is
- which slot owns it
- what enclosing function, block, or `let` it belongs to
- whether one node lies inside another node's subtree

Likely shape:

```text
node id -> kind, parent id, owning slot, nearest fn, nearest stmt, nearest let
```

TODO:

- add a tree-context builder in `draxl-merge`
- keep it derived from the base AST, not persisted in source
- make subtree and owner queries cheap enough for rule evaluation

Open question:

- how far can the owner-and-region model stretch before wider semantic
  relations need a second abstraction?

### 2. Binding Awareness

The motivating Git example is about a binding rename and a change to that
binding's meaning.

That means `draxl-merge` needs at least a narrow binding-aware layer, even if
it is much smaller than full name resolution.

Initial need:

- identify `let` bindings and params
- identify the reference set that points at those bindings
- identify the rename closure for a binding

TODO:

- add a first pass that resolves simple local bindings within a function
- start with parameters and identifier-pattern `let` bindings
- keep the scope intentionally narrow until the first semantic rules are proven

Open question:

- should this stay entirely inside `draxl-merge`, or should a reusable
  name-resolution helper crate exist later if more features need it?

### 3. Meaning-Defining Zones

Not every edit near a binding is a semantic conflict.

We need a small policy layer that says which AST regions change meaning enough
to require review.

Early examples:

- `let.init`
- `param.ty`
- `fn.ret`
- expression operator changes
- expression callee changes

TODO:

- define a first registry of meaning-defining regions for the current Rust
  profile
- keep the registry explicit and profile-backed, not heuristic-only
- attach each semantic rule to one or more named region kinds

### 4. Rule Model For Semantic Conflicts

Hard conflicts already have structured explanations. Semantic conflicts should
use the same discipline.

Each semantic rule should have:

- a stable conflict code
- a detector
- an explanation builder
- tests

Likely examples:

- `binding_rename_vs_meaning_change`
- `signature_change_vs_body_change`
- `operator_change_vs_operand_change`

TODO:

- extend the merge model to carry `ConflictClass::Semantic`
- require every new conflict code to ship with explanation text from the start
- avoid adding one-off semantic rules without a stable code and test coverage

### 5. Better Fixtures

The Git reproduction test proves motivating cases on the text side.

We still need the corresponding Draxl-side fixtures:

- one base `.rs.dx` file
- one left patch stream
- one right patch stream
- one expected semantic conflict report

TODO:

- add Draxl fixtures for the verified Git merge examples
- add semantic-merge tests that use the fixtures directly
- keep each fixture small enough to read quickly

### 6. Performance And Incremental Strategy

This is a later concern, but it should stay in view.

If semantic conflict checking grows, we may eventually want:

- cached tree indexes
- reusable binding summaries
- profile-specific conflict metadata

Default answer for now:

- do not build persistence or caching first
- measure after a few real semantic rules exist

## Suggested Work Order

1. Add tree-context indexing in `draxl-merge`.
2. Add narrow binding awareness for params and simple `let` bindings.
3. Add the first semantic rule:
   `binding_rename_vs_meaning_change`.
4. Add Draxl fixtures for the verified Git examples.
5. Add more semantic rules only after the first ones produce useful output.

## Acceptance Bar For Early Semantic Rules

Before calling the first few semantic rules solid, they should:

- detect the motivating examples
- explain why the conflict is semantic rather than hard
- point to the relevant binding, parameter, function, or region
- avoid firing on the existing clean ranked-insert example
