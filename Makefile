.PHONY: test retest week dry fmt clippy

test:
	cargo test --all-targets

retest: test test

fmt:
	cargo fmt --all

clippy:
	cargo clippy --all-targets -- -D warnings

week:
	cargo run -- week --fixtures fixtures/happy-week --out runs/latest

dry:
	cargo run -- week --fixtures fixtures/happy-week --out runs/dry --dry-run
