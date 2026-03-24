# Merge Docs

This directory collects the public docs for Draxl merge analysis, conflict
layering, and semantic conflict reporting.

Use this hub when you need to answer one of these questions:

- how does Draxl classify merge issues?
- what is a hard conflict versus a semantic conflict?
- where does build or CI validation sit in the merge pipeline?
- what merge-analysis work is still planned?

## Merge Model

Draxl merge analysis is layered.

It treats deterministic replay, validation or build success, and semantic
coherence as separate questions instead of collapsing them into one generic
"merge conflict" bucket.

## Guide

- [conflicts.md](conflicts.md): conflict layering, hard conflicts, and where
  validation or CI sits in the flow
- [semantic-conflicts.md](semantic-conflicts.md): current semantic-conflict
  rules, examples, and design direction
- [../todo/merge-conflicts.md](../todo/merge-conflicts.md): implementation
  notes and follow-up engineering work for merge analysis

## Core Terms

- hard conflict: patch streams are not jointly replayable in one deterministic
  way
- semantic conflict: patch streams replay cleanly but the merged result loses
  semantic coherence
- merge context: derived structural or semantic information used during
  conflict analysis
- conflict report: structured explanation for humans and agents

## Related Docs

- [../patching/README.md](../patching/README.md): patch streams and patch
  operators
- [../architecture.md](../architecture.md): crate-level architecture and where
  `draxl-merge` fits
- [../syntax.md](../syntax.md): Draxl source syntax and metadata prefixes
