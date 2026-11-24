# Data pipeline (SPEC_SIGNATURE_INDEX.md). Requires WordNet at WN_DIR or /tmp/wn/dict.

PYTHON ?= python3
MIN_ENTRIES ?= 35000
WN_DIR ?= /tmp/wn/dict

.PHONY: data
data:
	$(PYTHON) scripts/enrich_wordnet.py --mode wordnet-full \
		--emit-manifest data/words.manifest.json \
		--output data/words.json
	@test $$( $(PYTHON) -c "import json; print(len(json.load(open('data/words.json'))))" ) -ge $(MIN_ENTRIES) \
		|| (echo "entry count below MIN_ENTRIES=$(MIN_ENTRIES); see SPEC §13.6" >&2; exit 1)
	$(PYTHON) scripts/build_signature_index.py \
		--input data/words.json \
		--output data/signature_index.json
	$(PYTHON) scripts/build_affix_patterns.py \
		--input data/words.json \
		--output data/affix_patterns.json

.PHONY: data-index
data-index:
	$(PYTHON) scripts/build_signature_index.py \
		--input data/words.json \
		--output data/signature_index.json
	$(PYTHON) scripts/build_affix_patterns.py \
		--input data/words.json \
		--output data/affix_patterns.json

.PHONY: test
test:
	cargo test