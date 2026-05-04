.PHONY: build test

data/seagull.json: build/seagull.json
	cp build/seagull.json data/seagull.json

sentence:
	@uv run scripts/random_sentence.py

all: \
	build/cmucrossref.json \
	build/cmucrossref_onesyl.json \
	build/base_vocab_analysis.json \
	data/base_vocab.json \
	build/irregular_vocab.json \
	build/regular_vocab.json \
	build/obvious_outlines.json \
	build/reasonable_outlines.json \
	build/cluster_usages.json \
	build/seagull.json \

build/theory.json: scripts/validate_theory.py theory/*.md
	uv run scripts/validate_theory.py

build/cmucrossref.json: scripts/cmucrossref.py
	uv run python scripts/cmucrossref.py

build/cmucrossref_onesyl.json: scripts/onesyl.py build/cmucrossref.json
	uv run python scripts/onesyl.py

build/base_vocab_analysis.json: scripts/base_vocab_analysis.py
	uv run python scripts/base_vocab_analysis.py

data/base_vocab.json: build/cmucrossref.json
	jq keys build/cmucrossref.json > data/base_vocab.json

build/irregular_vocab.json build/regular_vocab.json: scripts/group_inflected_vocab.py build/base_vocab_analysis.json
	uv run scripts/group_inflected_vocab.py

build/obvious_outlines.json build/reasonable_outlines.json: scripts/obvious_outlines.py build/regular_vocab.json
	uv run scripts/obvious_outlines.py

build/cluster_usages.json: scripts/cluster_usages.py
	uv run scripts/cluster_usages.py

build/seagull.json: \
	build/theory.json \
	scripts/build_seagull.py \
	data/seagull_base.json \
	data/seagull_misc.json \
	build/obvious_outlines.json \
	build/reasonable_outlines.json \
	data/punctuation.json \
	data/commands.json
	uv run scripts/build_seagull.py

build:
	cargo build
	uv run maturin develop

test:
	cargo test
	uv run python scripts/examples.py

ime:
	$(MAKE) -C crates/seagull-ime build

clean:
	rm -rf build
	cargo clean
