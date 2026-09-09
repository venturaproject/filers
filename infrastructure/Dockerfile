# ─────────────────────────────────────────────────────────────────────────────
# Production image for the Rust API. Multi-stage: a fat build image compiles a
# static-ish release binary, a slim runtime image ships only that binary.
# ─────────────────────────────────────────────────────────────────────────────

# ---- build ----
FROM rust:1.92-slim AS build

WORKDIR /app

# Prime the dependency cache: build with stub sources so a change to our own
# code doesn't re-download / re-compile every crate.
COPY Cargo.toml Cargo.lock ./
RUN mkdir src \
    && echo 'fn main() {}' > src/main.rs \
    && echo '' > src/lib.rs \
    && cargo build --release \
    && rm -rf src

COPY src ./src
# Bust the stale stub artifacts, then build for real.
RUN touch src/main.rs src/lib.rs \
    && cargo build --release --bin server \
    && strip target/release/server

# ---- runtime ----
FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends curl \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --home-dir /app appuser \
    && mkdir -p /data/uploads \
    && chown -R appuser:appuser /data

WORKDIR /app
COPY --from=build /app/target/release/server /usr/local/bin/server

ENV PORT=8000 \
    BATCH_BASE_DIR=/data/uploads \
    RUST_LOG=rust_api=info,tower_http=info

USER appuser
EXPOSE 8000

ENTRYPOINT ["server"]
