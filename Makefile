.PHONY: test retest week dry watch fmt clippy

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

watch:
	cargo run -- watch --fixtures fixtures/competitor-watch --out runs/watch/latest
