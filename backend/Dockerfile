FROM rust:1.90-bookworm AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock* ./
COPY migrations ./migrations
COPY assets ./assets
COPY src ./src
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/var/cache/commercectrl-target \
    CARGO_TARGET_DIR=/var/cache/commercectrl-target cargo build --release --locked && \
    cp /var/cache/commercectrl-target/release/commercectrl-backend /app/commercectrl-backend

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/commercectrl-backend /usr/local/bin/commercectrl-backend
ENV RUST_LOG=commercectrl_backend=info,tower_http=info
EXPOSE 8080
HEALTHCHECK --interval=10s --timeout=3s --retries=5 CMD curl -fsS http://127.0.0.1:8080/health || exit 1
CMD ["commercectrl-backend"]
