# Patch Bundles

This page describes how to treat a patch stream as one logical bundle during
review, replay, and audit.

## Patch Stream As Bundle

The executable unit in the current patch surface is a textual patch stream: one
or more patch ops applied in order.

Use "bundle" as a documentation and workflow term for a patch stream when the
ops belong to one coherent intent, such as a rename plus the nearby structural
updates that keep the program consistent.

If several ops must happen together, keep them in the same stream and preserve
their execution order.

## `//` Rationale Comments

Patch streams may include whole-line `//` comments between ops.

Those comments are:

- non-semantic
- ignored by parsing, resolution, and execution
- useful for local rationale, sequencing notes, and review hints

They are not Draxl source comments, they do not become AST nodes, and they do
not survive as typed patch metadata after parsing.

Example:

```text
// Rename the public entrypoint before updating the doc payload.
set @f1.name = add_one_fast

// Keep the doc node, but clear the stale text for a follow-up rewrite.
clear @d1.text

replace @e2: (@e2 x * @l2 2)
```

## Bundles And Git Commits

A patch stream and a Git commit solve different problems:

- a patch stream expresses semantic edit instructions over the Draxl model
- a Git commit records repository history and the surrounding file changes

Current documented convention:

- use the commit message for the high-level narrative of the change
- use patch `//` comments for fine-grained rationale near specific ops or op
  groups
- keep unrelated semantic edits in separate patch streams when possible

The current public docs do not define a dedicated persisted commit schema,
typed patch-bundle wrapper, or structured per-op rationale metadata beyond
whole-line `//` comments in the stream.

## Review Guidance

Prefer a single patch stream when:

- the ops serve one intent
- order matters
- a reviewer benefits from seeing the rationale inline

Prefer separate streams or separate commits when:

- the changes are independent
- the rationale is unrelated
- replay or audit would be clearer if the edits are split
