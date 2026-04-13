.PHONY: build test

all: build/cmucrossref.json build/base_vocab_analysis.json data/base_vocab.json

build/cmucrossref.json: scripts/cmucrossref.py
	uv run python scripts/cmucrossref.py

build/base_vocab_analysis.json: scripts/base_vocab_analysis.py
	uv run python scripts/base_vocab_analysis.py

data/base_vocab.json: build/cmucrossref.json
	jq keys build/cmucrossref.json > data/base_vocab.json

build:
	cargo build
	uv run maturin develop

test:
	cargo test
	uv run python scripts/examples.py
