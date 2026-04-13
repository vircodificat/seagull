.PHONY: build test

build:
	cargo build
	uv run maturin develop

test:
	cargo test
	uv run python scripts/examples.py
