# Conflicts

Draxl should classify concurrent changes in layers.

Each layer answers a different question:

1. Can the patch streams be merged deterministically?
2. Does the merged result validate and pass project build or CI checks?
3. Does the merged result still preserve coherent meaning?

That layering keeps the conflict model narrow at each step and avoids pushing
compiler or CI failures into the semantic-conflict layer.

## Layer 1: Hard Conflicts

Hard conflicts come first.

A hard conflict means the patch streams are not jointly replayable in a
deterministic way against the same base.

Typical hard conflicts include:

- both sides writing the same node, scalar path, or single-child slot
- both sides inserting at the same ranked position
- one side rewriting or removing a subtree that the other side edits inside
- replay succeeding in one order but not the other
- both replay orders succeeding but producing different final ASTs

This layer is structural. It should use strict rules and avoid policy or
meaning-level inference.

## Layer 2: Validation, Build, and CI

After hard conflicts are cleared, Draxl should check whether the merged result
is valid in the language profile and acceptable to the target project.

That includes checks such as:

- AST or profile validation
- lowering validity
- compiler or type-checker success
- project build success
- test or CI success when available

This layer matters because many suspicious merges are already invalid at the
language or project level. If the compiler, type system, or CI rejects the
merge, that failure belongs here rather than in semantic conflict detection.

## Layer 3: Semantic Conflicts

Semantic conflicts come after the merge is structurally valid and the build or
CI layer accepts it.

A semantic conflict occurs when two patch streams commute structurally but not
semantically.

In practice, that means:

- both sides edit different meaning-bearing regions of the same semantic object
- the edits are structurally mergeable
- validation and build checks still pass
- the combined result no longer preserves a coherent meaning, contract, or
  interpretation

This layer should focus on compiler-clean but meaning-dirty merges.

Good candidates include:

- a binding name change on one side and a meaning change for the same binding
  on the other
- a parameter contract change on one side and body logic that still interprets
  the old contract on the other
- a callee contract change on one side and an argument representation that
  still follows the old contract on the other
- an effect or annotation change on one side and implementation behavior that
  no longer matches it on the other

The current detailed semantic-conflict examples are documented in
[semantic-conflicts.md](semantic-conflicts.md).

## Agent JSON Surface

`draxl-merge` and the root `draxl` facade now expose merge conflict reports as
machine-oriented JSON.

The top-level shape is:

- `conflicts`

Each conflict includes:

- `class`
- `code`
- `owner`
- `left_regions`
- `right_regions`
- `left`
- `right`

For semantic conflicts:

- `owner` identifies the semantic object being reviewed
- `left_regions` and `right_regions` identify the meaning-bearing regions each
  patch stream touched

For hard conflicts in the current profile:

- `owner` is `null`
- `left_regions` and `right_regions` are empty arrays
- agents should route on `code` plus the left/right operation evidence

## Why The Layering Matters

This order sharpens what semantic conflicts should cover.

Semantic conflicts should not become a fallback bucket for:

- hard replay failures
- type mismatches the compiler already rejects
- project policy failures that CI already rejects

Instead, semantic conflicts should report merged results that pass the earlier
layers but still carry a broken or misleading interpretation.

## Current Direction

Today the repository has:

- hard-conflict analysis in `draxl-merge`
- semantic rules for `let` binding rename versus initializer change
- semantic rules for parameter contract versus body interpretation
- Git reproductions showing that text merge can succeed without surfacing the
  underlying semantic overlap

As more semantic rules are added, they should be evaluated against this same
layering model.
