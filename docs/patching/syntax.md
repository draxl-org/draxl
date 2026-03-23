# Patch Syntax

This document defines the canonical textual syntax for Draxl patch streams and
semantic patch operations.

For operator behavior and the supported execution boundary, see
[semantics.md](semantics.md). For patch stream conventions and `//` rationale
comments, see [bundles.md](bundles.md).

## Status

This is the canonical notation for docs, logs, patch streams, the Rust API, and
the CLI.

## Grammar

```text
stream      := (layout* op layout*)*
layout      := blank_line | comment_line
comment_line:= ws* "//" text? newline
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
value       := ident | string | int | "true" | "false"
```

Whole-line `//` comments are non-semantic patch notes. They are ignored by
parsing, resolution, and execution, so they are safe for human rationale inside
a patch bundle.

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
- fragment parsing respects balanced parens, braces, and brackets across
  multiple lines
- blank lines and whole-line `//` patch comments may separate patch ops in a stream

## Behavior

This page defines surface shape, not full operator behavior. For `replace`,
`put`, `move`, `attach`, `detach`, and the rest of the semantic model, see
[semantics.md](semantics.md).

## Examples

```text
// Rename the API entrypoint before updating docs.
set @f1.name = add_one_fast

// Keep the doc node, but clear the stale text for a later rewrite.
clear @d1.text

replace @e2: (@e2 x * @l2 2)

insert @f1.body[ah]: @s4 @e4 trace();

put @f1.ret: @t9 i128

move @s4 -> @f1.body[ai]

attach @d2 -> @f1

detach @d2
```

## Current implementation boundary

The current Rust profile parses, resolves, and executes this textual syntax
through `draxl-patch`, the root `draxl` facade, and `draxl patch`.
