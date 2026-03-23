# Semantic Conflicts

This document describes layer 3 from [conflicts.md](conflicts.md).

Draxl uses two different conflict classes during merge analysis:

- **hard conflicts** stop deterministic auto-merge
- **semantic conflicts** are structurally mergeable but still require review

A hard conflict means the two patch streams cannot be replayed cleanly in a
deterministic way. A semantic conflict means the patch streams can still replay,
but the merged result may hide a meaning shift that should not pass silently.

## General Rule

A semantic conflict occurs when two patch streams commute structurally but not
semantically.

In practice, Draxl should look for cases where:

- both branches edit different meaning-bearing regions of the same semantic
  object
- the edits are individually valid and structurally mergeable
- the combined result changes multiple aspects of that object in a
  non-independent way
- the merged object no longer preserves a coherent meaning, contract, or
  interpretation

Typical semantic objects include:

- a binding, with regions such as its name, type, initializer, and uses
- a function, with regions such as its signature, return type, effects, and
  body
- an expression, with regions such as its operator, callee, and operands

## Why they matter

Text-based merges often miss meaning-level overlaps.

In an agent-heavy workflow, that gap gets worse:

- a merge may succeed even though two edits are coupled
- the resulting code may still compile and look tidy
- the combined result may carry misleading names, contracts, or assumptions
- agents need explicit feedback about why a review is required

The point of semantic conflict reporting is not to block every merge. The point
is to surface merged results that are structurally valid but still suspicious.

## Current Example

The first implemented semantic conflict rule is covered by:

- [crates/draxl-merge/tests/semantic_conflicts.rs](../crates/draxl-merge/tests/semantic_conflicts.rs)

Starting Draxl source:

```text
@m1 mod demo {
  @f1[a] fn price(@p1[a] amount: @t1 Cents) -> @t2 Cents {
    @s1[a] let @p2 subtotal = @e1 amount;
    @s2[b] @e2 subtotal
  }
}
```

Left patch stream:

```text
set @p2.name = subtotal_cents
replace @e2: @e2 subtotal_cents
```

Right patch stream:

```text
replace @e1: @e1 to_dollars(@e3 amount)
```

Those edits are structurally mergeable. There is no hard conflict.

But they are still review-worthy together:

- the left side renames the binding to `subtotal_cents`
- the right side changes the initializer so the same binding now carries
  `to_dollars(amount)`

That yields a merged result with a misleading binding name:

```text
@m1 mod demo {
  @f1[a] fn price(@p1[a] amount: @t1 Cents) -> @t2 Cents {
    @s1[a] let @p2 subtotal_cents = @e1 to_dollars(@e3 amount);
    @s2[b] @e2 subtotal_cents
  }
}
```

This example is one instance of a broader semantic-conflict pattern:

```text
one side changes one meaning-bearing projection of a semantic object
the other side changes another projection of the same object
the combined result remains structurally valid but loses semantic coherence
```

## Relation To The Git Reproduction

The text-side motivation for this work is captured by:

- [tests/git_merge.rs](../tests/git_merge.rs)

That test shows a plain Git merge succeeding without conflict even though the
merged result combines two semantically coupled edits. The Draxl semantic
conflict rule exists to make that class of issue visible at the structural
patch level.

## Current Scope

Today the implemented semantic rule is intentionally narrow:

- a `let` binding rename on one side
- an initializer-region change for the same `let` on the other side

This rule is a small starting point, not the final design.

Future semantic rules will likely cover cases such as:

- signature change versus body change
- operator change versus operand edits
- parameter rename versus type or contract changes

## Design Direction

Semantic conflicts should always ship with rich explanations.

Each rule should report:

- what the two sides changed
- why the overlap is semantic rather than hard
- which binding, node, or region is involved
- what the reviewer should look at next

AI agents especially need that explanation. A semantic conflict is only useful
if the system explains why it was raised.
