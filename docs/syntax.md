# Syntax

The current Draxl Rust profile uses explicit metadata
prefixes on supported syntax nodes.

## Metadata prefix

The canonical metadata form is:

```text
@id[rank]->anchor
```

Supported compact forms:

```text
@x1
@x1[a]
@x1->y2
@x1[a]->y2
```

## Meaning

### `@id`

`@id` gives the next supported node a stable identity.

Examples:

- items
- fields
- enum variants
- parameters
- statements
- modeled expressions
- doc comments
- line comments

Ids are opaque strings. They do not encode semantics beyond uniqueness within a
file.

### `[rank]`

`[rank]` orders siblings inside ranked slots.

Examples of ranked slots in the current prototype:

- module items
- struct fields
- enum variants
- function params
- block statements
- match arms

Ranks are opaque strings compared lexicographically. The prototype does not
impose a numeric scheme.

### `->anchor`

`->anchor` attaches detached docs or comments to an existing sibling node id.

Anchors are only needed when simple adjacency is not enough.

## Example

```rust
@m1 mod demo {
  @d1 /// Add one to x.
  @f1[a] fn add_one(@p1[a] x: @t1 i64) -> @t2 i64 {
    @c1 // Cache the intermediate value.
    @s1[a] let @p2 y = @e1 (@e2 x + @l1 1);
    @s2[b] @e3 y
  }
}
```

## Attachment rules

### Implicit attachment

Doc comments and line comments attach implicitly to the next semantic sibling
when `->anchor` is absent.

### Explicit attachment

Use `->anchor` when a doc or comment is detached from its target in the source
layout.

### Validation behavior

The validator rejects:

- anchors that do not refer to an existing node id
- docs/comments that are detached without a following sibling or explicit
  anchor

## Supported subset

The current Rust profile prototype supports:

- `mod`
- `use`
- `struct`
- `enum`
- `fn`
- parameters
- path types
- integer and string literals
- blocks
- `let`
- expression statements
- path expressions
- grouped expressions
- call expressions
- binary expressions with `+`, `-`, and `<`
- unary minus
- `match`
- match arms
- `use` trees with groups and globs
- doc comments
- line comments

## Canonical formatting

The canonical printer preserves ids, ranks, and anchors while making source
order deterministic.

Canonical printing always emits metadata in this order:

```text
@id[rank]->anchor
```

That stability matters because Draxl is intended to be edited repeatedly by
tools, not only read once by a compiler front end.
