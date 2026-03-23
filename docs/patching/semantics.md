# Patch Semantics

Draxl patching is structural, not textual.

Patch operations address stable node ids, schema-defined slot names, attachment
relations, and scalar field paths. That makes patch application resilient to
surrounding formatting changes and explicit about semantic intent.

For the canonical textual notation used in docs, the Rust API, and the CLI, see
[syntax.md](syntax.md). For patch stream conventions and `//` rationale
comments, see [bundles.md](bundles.md). This document focuses on the semantic
model and the current Rust API behavior.

Textual patch streams may also include whole-line `//` comments between ops.
Those comments are non-semantic notes for review or rationale and are discarded
by resolution and execution.

## Core model

The public patch surface splits into four families:

- node-oriented: `replace`, `delete`, `move`
- slot-oriented: `insert`, `put`
- relation-oriented: `attach`, `detach`
- path-oriented: `set`, `clear`

The current Rust API mirrors that split through structured `PatchOp` values,
`SlotRef`, `RankedDest`, `PatchDest`, `PatchPath`, and `PatchValue`.

## Addressing

### Node ids

`@id` names a node identity in the current tree. The executor resolves the id
and learns the node kind from the Draxl schema plus AST lookup.

### Slot refs

A slot ref names a child slot owned by either the file root or a node.

Examples in the current profile:

- `file.items`
- `@m1.items`
- `@f1.params`
- `@f1.body`
- `@f1.ret`
- `@let1.init`
- `@e7.arms`

Public slot names come from the Draxl profile. They are not required to match
Rust struct field names used by the current implementation.

### Paths

Paths address scalar fields.

Examples in the current profile:

- `@f1.name`
- `@d1.text`
- `@e7.op`
- `@s2.semi`

## Semantics

### `replace`

`replace` is node-oriented and preserves the outer shell of the target node.

That means it keeps:

- the target id
- the same parent owner and slot
- the same outer rank when applicable
- the same outer anchor metadata when applicable
- the same inbound attachment set targeting that id

Conceptually:

```text
replace keeps the node shell and rewrites the node body
```

### `put`

`put` is slot-oriented. It sets the occupant of a single-child slot, whether
the slot was empty or already occupied.

If you want to preserve the existing node identity and attachment set, use
`replace` on the occupant node instead of `put`.

### `move`

`move` preserves node identity and relocates the node to a new destination.

Attachments are identity-bound, not slot-bound:

- moving a node carries its attached docs/comments
- cross-container moves rewrite attachment bookkeeping implicitly
- moves into contexts that cannot host the attachment closure are rejected

### `attach` and `detach`

These are first-class attachment operations. They rewrite anchor relations
under the same sibling/container constraints that validation enforces.

### `set` and `clear`

These update scalar fields through schema-defined paths instead of introducing
bespoke rename or metadata verbs.

## Current implementation boundary

The current executor supports the modeled Rust profile through both the
structured Rust API and the canonical textual patch surface.

Supported today:

- textual patch parsing and schema-backed resolution through `draxl-patch`,
  `draxl`, and `draxl patch`
- `insert` into ranked `items`, `fields`, `variants`, `params`, `body`, and
  `arms` slots
- `put` into the current modeled single-child slots such as `ret`, `ty`, `pat`,
  `init`, `expr`, `lhs`, `rhs`, `callee`, `scrutinee`, `guard`, and arm `body`
- `replace` of items, fields, variants, params, statements, match arms,
  expressions, types, patterns, and attachable trivia
- `delete` of ranked-slot children and optional single-child occupants that can
  be removed without leaving a required slot empty
- `move` for the same removable source regions, including attachment-closure
  transport
- `attach` and `detach` for doc/comment nodes
- `set` and `clear` over the current scalar subset: names, trivia text,
  operators, and statement semicolon state

Not implemented yet:

- profile metadata paths beyond the currently modeled scalar subset
- patching over AST regions that the current Rust profile does not model with
  stable ids or legal destination slots
