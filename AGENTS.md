# Repository Agent Instructions

## Scope and Precedence

- Instructions apply by directory scope.
- Use the nearest `AGENTS.md` for files in that subtree.
- Rules are additive when they do not conflict.
- The nearest `AGENTS.md` rule wins for conflicts inside its subtree.

## Task Lifecycle

- Treat each direct execution request (`do <number>`, `implement`, `fix`, `apply`) as one execution task.
- Edit the requested files, run relevant validation before commit when possible, and commit with `git commit --no-gpg-sign` after checks pass unless the user says `do not commit`.
- Run formatting when the changed files or project tooling make it relevant.
- If checks fail, report the failures and stop. Do not commit.
- Push only when explicitly requested.

## Validation

- Prefer the standard project validation command when the repository provides one.
- If a full-project check such as `make check` exists, run it before commit when possible.
- If validation cannot run in the current environment, say so before commit.

## Documentation Style

- Classify standalone lines like `That distinction matters.` as content-free emphasis, rhetorical filler, or throat-clearing, and remove them.
- Do not write standalone meta-emphasis or signposting sentences that only assert importance, contrast, or relevance without adding a concrete claim.
- Prefer affirmative definitions over negative ones. Do not introduce a concept with lines such as `X is not Y` or `The point is not to ...` when a direct positive statement would be clearer.
- Use explicit negative framing only in sections that are actually about boundaries, such as `Non-goals`, `Limits`, or `Rejected approaches`.
- Avoid filler lines such as `That distinction matters.`, `This matters.`, `This is important.`, or `This is the most important missing abstraction.`
- If a sentence can be deleted without losing technical content, delete it or fold the concrete reason into a nearby sentence.
- Prefer direct statements of mechanism or consequence.
  Example: write `Hard conflicts stop deterministic replay, while semantic conflicts require review.` instead of `That distinction matters.`
