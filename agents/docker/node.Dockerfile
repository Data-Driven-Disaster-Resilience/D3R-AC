FROM node:20-slim AS build
WORKDIR /app
COPY agents.config.yaml schemas/ scripts/ ./
COPY node/ node/
RUN apt-get update && apt-get install -y --no-install-recommends python3 python3-pip && \
    pip3 install --no-cache-dir --break-system-packages pyyaml jsonschema && \
    python3 scripts/build_manifest.py && \
    cd node && npm install && npm run build

FROM node:20-slim
WORKDIR /app
COPY --from=build /app/node ./node
WORKDIR /app/node
CMD ["npm", "run", "start", "--", "run-all"]
