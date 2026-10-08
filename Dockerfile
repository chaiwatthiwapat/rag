FROM rust:1.97-trixie AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release --locked

FROM debian:trixie-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates libgomp1 && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/rust-fastembed-rag /usr/local/bin/rag
ENV FASTEMBED_CACHE_DIR=/model-cache
ENTRYPOINT ["rag"]
