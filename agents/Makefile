.PHONY: build manifest install-python install-node build-python build-node test test-python test-node lint dev clean

# One command builds the whole fleet: validate manifest -> codegen -> install -> build both stacks.
build: manifest install-python install-node build-node
	@echo "d3rac-agents build complete."

manifest:
	pip install --quiet --break-system-packages pyyaml jsonschema 2>/dev/null || pip install --quiet pyyaml jsonschema
	python3 scripts/build_manifest.py

install-python:
	pip install --break-system-packages -e ./python[dev] 2>/dev/null || pip install -e ./python[dev]

install-node:
	cd node && npm install

build-python:
	python3 -c "import d3rac_agents; print('python package OK:', d3rac_agents.__version__)"

build-node:
	cd node && npm run build

test: test-python test-node

test-python:
	cd python && pytest -q

test-node:
	cd node && npm test

lint:
	cd node && npm run lint
	ruff check python/src || true

# Runs the full pipeline once against the file bus: hazard/exposure/vulnerability ->
# risk model -> coordination -> alerting -> contract trigger (stubbed).
dev: manifest
	python3 -m d3rac_agents.cli run-all
	cd node && npm run start -- run-all

clean:
	rm -rf data/bus/*.jsonl node/dist node/node_modules python/*.egg-info
