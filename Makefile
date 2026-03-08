fmt:
	cargo fmt --all

check:
	cargo fmt --all --check
	cargo test --workspace
