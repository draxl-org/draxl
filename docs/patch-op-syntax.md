# Patch Op Syntax

This document defines the textual syntax for Draxl semantic patch operations.

## Status

This is the canonical notation for docs, logs, and future tooling.

The current bootstrap implementation does not parse this syntax yet. Today the
Rust API exposes typed `PatchOp` enums and supports only `insert`, `replace`,
and `delete` over ranked slot children.

## Design rules

- operation names use `verb_kind` snake case such as `replace_expr` and
  `insert_stmt`
- targets are addressed by stable ids such as `@e2`
- inserts name a parent, slot, and rank explicitly instead of using `after` or
  `before` sibling shorthand
- replacement and insertion payloads embed Draxl source fragments
- the operation header owns the outer slot and rank; the fragment owns ids,
  inner ranks, and inner structure

## Canonical forms

```text
replace_<kind> @target with <fragment>

delete_<kind> @target

insert_<kind> into <parent>.<slot> rank=<rank>: <fragment>

attach_doc @doc -> @target
attach_comment @comment -> @target
```

## Kinds

Common kinds:

- `item`
- `field`
- `variant`
- `param`
- `stmt`
- `arm`
- `expr`
- `doc`
- `comment`

Future profiles may add more kinds, but the naming pattern stays the same.

## Addressing

### Target ids

Use `@id` to identify the node being replaced, deleted, or attached.

### Parent selectors

`<parent>` is either `file` for the root file slot or `@id` for a node-owned
slot.

Examples:

- `file.file_items`
- `@m1.items`
- `@f1.params`
- `@f1.body`
- `@e7.arms`

### Slot names

Slot names match the Draxl AST model:

- `file_items`
- `items`
- `fields`
- `variants`
- `params`
- `body`
- `arms`

## Fragments

The `<fragment>` is ordinary Draxl source for the inserted or replacement node.

Rules:

- the outer fragment must include its stable id
- the outer fragment omits its rank in canonical form; `insert_*` gets the
  outer rank from `rank=...`, while `replace_*` inherits the outer rank from
  the target node
- nested ranked children inside the fragment still carry their own `[rank]`
  metadata
- the fragment may introduce new ids; the target id is a locator, not an
  implicit promise that the replacement preserves the old id

## Examples

### Canonical forms

The `replace_expr` form below is part of the intended surface language. The
currently executable subset appears in the next section.

```text
replace_expr @e2 with (@e9 x * @l2 2)

insert_stmt into @f1.body rank=ah: @s4 @e4 trace();

delete_stmt @s2

attach_doc @d2 -> @f1
```

### Bootstrap subset examples

The current runtime supports ranked slot children only:

```text
replace_stmt @s2 with @s9 @e9 audit();

insert_stmt into @f1.body rank=ah: @s4 @e4 trace();

delete_stmt @s2
```

Root and nested item insertion use the same address form:

```text
insert_item into file.file_items rank=b:
  @f9 fn helper(@p9[a] x: @t9 i64) -> @t10 i64 {
    @s9[a] @e9 x
  }

insert_item into @m1.items rank=c:
  @f10 fn extra() {}
```

## Current implementation boundary

The current `draxl-patch` crate supports only these structural families:

- `insert_item`
- `insert_field`
- `insert_variant`
- `insert_param`
- `insert_stmt`
- `insert_arm`
- matching `replace_*`
- matching `delete_*`

Expression replacement, move operations, and attachment ops are part of the
intended surface language but are not implemented in the bootstrap patch engine
yet.
