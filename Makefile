.PHONY: fmt check publish-dry publish

fmt:
	cargo fmt --all

check:
	cargo fmt --all --check
	cargo test --workspace

publish-dry: check
	cargo publish -p draxl --dry-run

publish: check
	cargo publish -p draxl
