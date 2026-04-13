.PHONY: build test

all:
	uv run python scripts/cmucrossref.py

build:
	cargo build
	uv run maturin develop

test:
	cargo test
	uv run python scripts/examples.py
