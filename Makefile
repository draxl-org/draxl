.PHONY: fmt check itest itest-codex codex-setup publish-dry publish

ROOT ?= .

fmt:
	cargo fmt --all

check:
	cargo fmt --all --check
	cargo test --workspace

itest:
	cargo test -p draxl-itest

itest-codex:
	cargo test -p draxl-itest codex_ -- --ignored --nocapture

codex-setup:
	cargo run -p draxl-cli -- mcp setup --client codex --root "$(ROOT)"
	@printf 'Configured Codex for %s via `draxl mcp setup --client codex`.\n' "$(ROOT)"
	@printf 'Start Codex in that workspace and it will launch `draxl mcp serve` on demand.\n'

publish-dry: check
	cargo publish -p draxl --dry-run

publish: check
	cargo publish -p draxl
