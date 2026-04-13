.PHONY: build test

all: \
	build/cmucrossref.json \
	build/base_vocab_analysis.json \
	data/base_vocab.json \
	irregular_vocab.json \
	regular_vocab.json \
	build/obvious_outlines.json \
	build/reasonable_outlines.json

build/cmucrossref.json: scripts/cmucrossref.py
	uv run python scripts/cmucrossref.py

build/base_vocab_analysis.json: scripts/base_vocab_analysis.py
	uv run python scripts/base_vocab_analysis.py

data/base_vocab.json: build/cmucrossref.json
	jq keys build/cmucrossref.json > data/base_vocab.json

irregular_vocab.json: scripts/group_inflected_vocab.py
	uv run scripts/group_inflected_vocab.py

regular_vocab.json: scripts/group_inflected_vocab.py
	uv run scripts/group_inflected_vocab.py

build/obvious_outlines.json build/reasonable_outlines.json: scripts/obvious_outlines.py build/regular_vocab.json
	uv run scripts/obvious_outlines.py

build:
	cargo build
	uv run maturin develop

test:
	cargo test
	uv run python scripts/examples.py
