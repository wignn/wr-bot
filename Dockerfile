FROM rust:1.81-slim AS builder
LABEL authors="tigfi"

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY Cargo.toml Cargo.lock ./

RUN mkdir src && \
    echo "fn main(){}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

COPY . .

RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl-dev \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

RUN update-ca-certificates

RUN useradd -m -u 1001 appuser

WORKDIR /app

COPY --from=builder /app/target/release/worm /app/worm

RUN chown -R appuser:appuser /app
