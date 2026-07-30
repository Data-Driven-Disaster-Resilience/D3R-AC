FROM python:3.11-slim
WORKDIR /app
COPY agents.config.yaml schemas/ scripts/ ./
COPY python/ python/
RUN pip install --no-cache-dir pyyaml jsonschema redis && \
    python3 scripts/build_manifest.py && \
    pip install --no-cache-dir -e ./python
CMD ["python3", "-m", "d3rac_agents.cli", "run-all"]
