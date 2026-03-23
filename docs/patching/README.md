# Patch Docs

This directory collects the public docs for Draxl semantic patch streams and
patch operators.

Use this hub when you need to answer one of these questions:

- how do I write a patch stream?
- what does each patch op mean?
- how do `//` patch comments work?
- how should a patch stream relate to a Git commit or review bundle?

## Patch Surface

Draxl patching is structural, not textual. A patch stream names semantic
targets directly through stable node ids, slot names, attachment relations, and
scalar paths instead of relying on line-oriented diff context.

The textual patch surface is executable today through the Rust API and
`draxl patch` for the current modeled Rust profile.

## Guide

- [syntax.md](syntax.md): canonical textual grammar, addressing, fragment
  rules, and examples
- [semantics.md](semantics.md): operator families, behavior, and the current
  implementation boundary
- [bundles.md](bundles.md): patch streams as logical bundles, `//` rationale
  comments, and how patch bundles relate to Git commits
- [implementation-todo.md](implementation-todo.md): implementation notes and
  follow-up engineering work for the patch frontend

## Core Terms

- patch op: one semantic edit instruction such as `replace`, `insert`, or
  `attach`
- patch stream: one or more patch ops in textual execution order
- patch bundle: documentation term for a patch stream treated as one logical
  change during review, replay, or audit

Patch `//` comments live in the stream, not in the AST. They are human
annotations for review or rationale and are ignored by parsing, resolution, and
execution.

## Related Docs

- [../syntax.md](../syntax.md): Draxl source syntax and metadata prefixes
- [../architecture.md](../architecture.md): crate-level architecture and where
  patching fits
- [../merge/README.md](../merge/README.md): merge-analysis doc hub
- [../merge/conflicts.md](../merge/conflicts.md): conflict layering for patch
  streams
- [../merge/semantic-conflicts.md](../merge/semantic-conflicts.md): current
  semantic conflict examples
