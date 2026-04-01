.PHONY: fmt check itest itest-codex publish-dry publish

fmt:
	cargo fmt --all

check:
	cargo fmt --all --check
	cargo test --workspace

itest:
	cargo test -p draxl-itest

itest-codex:
	cargo test -p draxl-itest codex_ -- --ignored --nocapture

publish-dry: check
	cargo publish -p draxl --dry-run

publish: check
	cargo publish -p draxl
