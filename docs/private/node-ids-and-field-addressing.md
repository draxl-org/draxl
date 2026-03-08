# Node IDs and Field Addressing

This note records a design decision about expression identity and operator
editing in Draxl.

## Decision

Draxl should identify semantic nodes with stable IDs and address internal fields
of those nodes with selectors.

Default rule:

- IDs belong on semantic nodes.
- Operators are fields on owning expression nodes, not separate ID-bearing nodes.
- Token-level IDs are only needed when a token must carry independent metadata
  or attachment.

## Example

In the current example:

```text
@s1[a] let @p2 y = @e1 (@e2 x + @l1 1);
```

`@e2` already identifies the binary expression node `x + 1`.

Conceptually:

```text
ExprBinary {
  id: e2,
  lhs: x,
  op: Add,
  rhs: l1,
}
```

The missing capability is not an ID on `+`. The missing capability is a patch
or selector surface that can address the operator field of `@e2`.

## Recommended model

Use this split by default:

- docs, comments, and similar independently attachable metadata carriers get
  their own IDs
- operators, visibility, mutability, names, and similar properties remain
  fields on owning nodes

This keeps Draxl AST-native and avoids sliding toward CST or token identity in
the bootstrap design.

## Implications for patching

Future patching should support stable node IDs plus typed field paths.

Examples:

```text
set @e2.op = Mul
replace @e2.rhs with @l2 2
```

Or with more explicit operation names:

```text
set_binop @e2 *
replace_expr @e2 (@e3 x * @l2 2)
```

The important property is that `@e2.op` is a validated field on an
`ExprBinary`, not an arbitrary dotted string.

## Why not give operators IDs by default

Giving every operator token its own ID makes the surface noisier and pushes the
model toward token-level identity too early.

That trade only makes sense when operator tokens need independent:

- attachment
- metadata
- provenance
- review or audit references

Until then, the binary or unary expression node is the correct identity unit.

## Status

This is a design note, not an implemented feature.

Current state:

- binary and unary expressions carry optional metadata
- operators are enum fields on those expression nodes
- patch docs currently describe node-level replacement, not field-level edits

If public docs are updated later, `docs/syntax.md` should clarify that `@e2`
identifies the binary expression node, and patch docs should introduce typed
field addressing if that surface is adopted.
