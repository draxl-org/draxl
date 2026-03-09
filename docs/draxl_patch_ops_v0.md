# Draxl Patch Ops v0

This document defines a compact textual syntax for Draxl semantic patch operations.

## Status

This is the proposed canonical surface for docs, logs, patch streams, and future tooling.

The current bootstrap implementation still exposes typed Rust `PatchOp` enums and supports only ranked-slot `insert`, `replace`, and `delete` over a narrow subset. The syntax below is the intended regular surface.

## Design goals

- one operation per line
- a small fixed verb set
- node kind inferred from the target id or destination slot
- one destination grammar for insert and move
- `replace` preserves the outer node identity
- fragments use ordinary Draxl source
- scalar fields and future metadata use path updates instead of bespoke verb families
- avoid repeated kind names and redundant keywords

## Core idea

The old shape repeats kind information in the verb:

```text
replace_expr @e2 with (...)
insert_stmt into @f1.body rank=ah: @s4 ...
delete_stmt @s2
attach_doc @d2 -> @f1
```

The slot schema and target ids already carry that information. The compact surface removes the repetition:

```text
replace @e2: (...)
insert @f1.body[ah]: @s4 ...
delete @s2
attach @d2 -> @f1
```

This keeps the language smaller, more regular, and cheaper to generate.

## Canonical grammar

```text
op        := insert | put | replace | delete | move | set | clear | attach | detach

insert    := "insert" ranked_dest ":" fragment
put       := "put" slot_ref ":" fragment
replace   := "replace" node_ref ":" fragment
delete    := "delete" node_ref
move      := "move" node_ref "->" dest
set       := "set" path "=" value
clear     := "clear" path
attach    := "attach" node_ref "->" node_ref
detach    := "detach" node_ref

dest      := ranked_dest | slot_ref
ranked_dest := slot_ref "[" rank "]"
slot_ref  := owner "." slot
owner     := "file" | node_ref
path      := node_ref ("." ident)+
node_ref  := "@" ident
```

## Addressing

### Node refs

Use `@id` to identify an existing node.

Examples:

- `@f1`
- `@s2`
- `@e7`

### Slot refs

A slot ref names a child slot owned by either the root file or a node.

Examples:

- `file.items`
- `@m1.items`
- `@f1.params`
- `@f1.body`
- `@e7.arms`
- `@let1.init`

Use `insert` for ranked slots and `put` for single-child slots.

### Ranked destinations

A ranked destination names a slot and the explicit rank to assign to the inserted node.

Examples:

- `file.items[b]`
- `@m1.items[c]`
- `@f1.body[ah]`
- `@e7.arms[d]`

### Field paths

A path addresses a scalar field or a future metadata key attached to a node.

Examples:

- `@e7.op`
- `@f1.name`
- `@f1.vis`
- `@f1.meta.owner`
- `@f1.meta.review.required`

## Slot naming

Slot names come from the Draxl AST schema. Common names include:

- `items`
- `fields`
- `variants`
- `params`
- `body`
- `arms`
- `ret`
- `init`
- `cond`
- `else`

Profiles may add more slot names. The operation vocabulary stays fixed.

## Fragments and values

### Insert fragments

`insert` fragments must include the outer node id and omit the outer rank.

Example:

```text
insert @f1.body[ah]: @s4 @e4 trace();
```

The inserted statement gets rank `ah` from the destination. Inner ranked children still carry their own ranks.

### Put fragments

`put` fills a single-child slot. The fragment includes the outer node id.

`put` is slot-oriented. If the slot is already occupied, the old occupant is
replaced. If the replacement needs to preserve the old occupant identity and
attachment set, use `replace` on that node instead of `put`.

Example:

```text
put @f1.ret: @t9 Result<@t10 i64, @t11 Error>
```

### Replace fragments

`replace` preserves the outer shell of the target node. The fragment omits the
outer node id and outer placement metadata.

Example:

```text
replace @e2: (@e9 x * @l2 2)
```

This means “rewrite the node `@e2` in place using this new subtree.”

The shell preserved by `replace` includes:

- the same outer id
- the same parent owner and slot
- the same outer rank where applicable
- the same outer anchor metadata where applicable
- the same inbound attachment set targeting `@id`

If a change needs a new outer id, express it as `delete` plus `insert`, or add a future dedicated identity operation.

### Set values

`set` updates scalar fields, enum tags, or future metadata values.

Typical values:

- bare identifiers for names and enum variants
- quoted strings for text
- numbers for numeric values
- `true` and `false` for booleans

Examples:

```text
set @e7.op = mul
set @f1.name = add_one_fast
set @f1.meta.owner = core_compiler
```

## Operation semantics

### `insert`

Add a new node to a ranked slot at an explicit rank.

```text
insert @f1.body[ah]: @s4 @e4 trace();
```

### `put`

Fill or replace a single-child slot.

```text
put @let1.init: @e9 compute();
put @f1.ret: @t9 i128
```

If the slot was already occupied, the old occupant is removed together with any
docs/comments anchored to that identity. Those attachments do not automatically
jump to the new node.

### `replace`

Rewrite an existing node in place while preserving its outer identity and position.

```text
replace @s2: @e9 audit();
replace @e2: (@e9 x * @l2 2)
```

The op header owns the outer shell. A replacement fragment must not carry
competing outer rank, slot, or anchor metadata.

### `delete`

Remove an existing node.

```text
delete @s2
```

### `move`

Relocate an existing node while preserving its id.

```text
move @s4 -> @f1.body[ai]
move @e9 -> @let1.init
```

Attachments are identity-bound, not slot-bound:

- moving a node carries its attachment closure
- cross-container moves rewrite attachment bookkeeping implicitly
- moves into destinations that cannot host that attachment closure are invalid

### `set`

Update a scalar field or metadata path.

```text
set @e7.op = add
set @f1.vis = pub
set @f1.meta.owner = core_compiler
```

### `clear`

Clear an optional scalar field or metadata path.

```text
clear @f1.vis
clear @f1.meta.owner
```

### `attach`

Attach a detached doc, comment, or other attachment-capable node to a target node.

```text
attach @d2 -> @f1
attach @c3 -> @s2
```

### `detach`

Remove an attachment edge while keeping the detached node.

```text
detach @d2
```

## Canonical examples

```text
replace @e2: (@e9 x * @l2 2)
insert @f1.body[ah]: @s4 @e4 trace();
delete @s2
attach @d2 -> @f1
```

Root and nested item insertion use the same destination form:

```text
insert file.items[b]: @f9 fn helper(@p9[a] x: @t9 i64) -> @t10 i64 { @s9[a] @e9 x }
insert @m1.items[c]: @f10 fn extra() {}
```

Field and metadata updates use `set` and `clear`:

```text
set @e7.op = mul
set @f1.name = add_one_fast
set @f1.meta.owner = core_compiler
clear @f1.meta.owner
```

Single-child slots use `put`:

```text
put @f1.ret: @t9 Result<@t10 i64, @t11 Error>
put @let1.init: @e9 compute();
```

Moves reuse the same destination syntax as inserts:

```text
move @s4 -> @f1.body[ai]
move @e9 -> @let1.init
```

## Migration from the current draft

Use these rewrites:

```text
replace_expr @e2 with (...)                  -> replace @e2: (...)
delete_stmt @s2                              -> delete @s2
insert_stmt into @f1.body rank=ah: @s4 ...  -> insert @f1.body[ah]: @s4 ...
attach_doc @d2 -> @f1                        -> attach @d2 -> @f1
attach_comment @c3 -> @s2                    -> attach @c3 -> @s2
```

Main changes:

- drop `verb_kind` names
- drop `into`, `with`, and `rank=`
- infer kind from schema and ids
- make `replace` identity-preserving by default
- use `set` and `clear` for scalar fields and future metadata

## Current implementation boundary

The current Rust executor supports this semantic model over the current modeled
Rust profile through structured `PatchOp` values. Parsing the textual syntax
above is still future work.

Supported today:

- `insert` into ranked slots
- `put` into the current modeled single-child slots
- `replace` across the current modeled node families, including expressions and
  types
- `delete` and `move` where removing the source does not leave a required slot
  empty
- `attach` and `detach` for doc/comment nodes
- `set` and `clear` over the current scalar subset
