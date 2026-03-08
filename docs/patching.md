# Patching

Draxl patching is structural, not textual.

The patch model operates on stable ids and named slots in the AST. That makes
patch application resilient to surrounding formatting changes and more explicit
about where a concurrent edit belongs.

For the textual command notation used in docs and future tooling, see
[patch-op-syntax.md](patch-op-syntax.md). This document focuses on the AST
model and the current Rust API behavior.

## Core model

The current bootstrap patch crate exposes three operations:

- `Insert`
- `Replace`
- `Delete`

These operate over ranked slot children rather than arbitrary subtrees.

## Parent and slot addressing

An insertion targets:

- a parent selector
- a slot name
- a rank for the inserted child

Example parent selectors:

- the file root
- a specific node id such as `f1`

Example slots in the current prototype:

- `file_items`
- `items`
- `fields`
- `variants`
- `params`
- `body`
- `arms`

## Insert

Insertions create a new child inside a slot under a specific parent.

Conceptually:

```text
insert node N into parent P slot S with rank R
```

The patch layer assigns slot metadata and rank metadata to the inserted node so
the validator and printer can treat it as a regular child.

## Replace

Replace swaps a ranked child identified by stable id while preserving the slot
and rank position of the original child.

Conceptually:

```text
replace child @target with replacement node N
```

This is useful when an agent wants to rewrite a specific function, statement,
parameter, or match arm without rebuilding the rest of the parent container.

## Delete

Delete removes a ranked child identified by stable id.

Conceptually:

```text
delete child @target
```

Deletion only applies to children in supported ranked slots.

## Why this matters

Text patches describe byte changes. Draxl patches describe semantic intent:

- which node is the parent
- which slot receives the edit
- which rank determines ordered placement
- which child id is replaced or deleted

That is a better fit for agent workflows and concurrent editing because the
patch is expressed in terms of the tree model rather than incidental text
layout.

## Current limits

The current patch layer is intentionally narrow:

- no move operation
- no rename-specific op
- no generalized subtree rewrite DSL
- only the modeled bootstrap subset is patchable

Those limits are acceptable for the prototype because the point is to prove the
identity and slot model first.
