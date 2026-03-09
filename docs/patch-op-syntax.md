# Patch Op Syntax

This document defines the canonical textual syntax for Draxl semantic patch
operations.

## Status

This is the canonical notation for docs, logs, patch streams, and future
tooling.

The current Rust executor already follows this semantic model through structured
`PatchOp` values. Parsing the textual surface itself is still future work.

## Grammar

```text
op          := insert | put | replace | delete | move | set | clear | attach | detach

insert      := "insert" ranked_dest ":" fragment
put         := "put" slot_ref ":" fragment
replace     := "replace" node_ref ":" fragment
delete      := "delete" node_ref
move        := "move" node_ref "->" dest
set         := "set" path "=" value
clear       := "clear" path
attach      := "attach" node_ref "->" node_ref
detach      := "detach" node_ref

dest        := ranked_dest | slot_ref
ranked_dest := slot_ref "[" rank "]"
slot_ref    := owner "." slot
owner       := "file" | node_ref
path        := node_ref ("." ident)+
node_ref    := "@" ident
```

## Addressing

### Node refs

Use `@id` to identify an existing node.

Ids are semantic locators. Kind inference comes from schema plus AST lookup,
not from id spelling.

### Slot refs

A slot ref names a profile-defined child slot owned by either the root file or
another node.

Examples:

- `file.items`
- `@m1.items`
- `@f1.params`
- `@f1.body`
- `@f1.ret`
- `@let1.init`
- `@e7.arms`

Use `insert` for ranked slots and `put` for single-child slots.

### Paths

A path addresses a scalar field.

Examples:

- `@f1.name`
- `@d1.text`
- `@e7.op`
- `@s2.semi`

## Fragments

Fragments use ordinary Draxl source.

Rules:

- `insert` and `put` fragments include the outer node id
- `insert` fragments omit the outer rank
- `put` fragments omit outer slot metadata
- `replace` fragments rewrite the node body and must not carry competing outer
  rank, slot, or anchor metadata
- `replace` preserves the target node identity and outer placement

## Semantics

### `replace`

`replace @id: ...` is node-oriented.

It preserves:

- the same outer id
- the same parent owner and slot
- the same outer rank where applicable
- the same outer anchor metadata where applicable
- the same inbound attachment set targeting that id

If you need a different outer identity, use `delete` plus `insert`, or `put`
when you are setting a single-child slot occupant.

### `put`

`put <owner>.<slot>: ...` is slot-oriented.

It sets the occupant of a single-child slot, whether the slot was empty or
already occupied. It may therefore replace an occupied slot with a new outer
node identity.

### `move`

Attachments are identity-bound, not slot-bound.

- `move` carries the moved node and its attachment closure
- cross-container moves rewrite attachment bookkeeping implicitly
- moves into contexts that cannot host the attachment closure are rejected

### `attach` and `detach`

These rewrite declared attachment relations under profile constraints. They are
not arbitrary graph-edge edits.

## Examples

```text
replace @e2: (@e9 x * @l2 2)

insert @f1.body[ah]: @s4 @e4 trace();

put @f1.ret: @t9 i128

move @s4 -> @f1.body[ai]

set @f1.name = add_one_fast

clear @d1.text

attach @d2 -> @f1

detach @d2
```

## Current implementation boundary

The current executor supports the semantic model above through the structured
Rust API over the modeled Rust profile. The textual syntax in this document is
canonical, but not parsed yet.
