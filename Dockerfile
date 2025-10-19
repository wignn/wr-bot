# Stage 1: Builder
FROM rust:1.83-slim AS builder

# Install dependencies yang diperlukan untuk build
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

COPY Cargo.toml Cargo.lock ./

COPY src ./src
COPY config.json system-prompt.txt ./

ENV CARGO_PROFILE_RELEASE_LTO=true
ENV CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1
ENV CARGO_PROFILE_RELEASE_OPT_LEVEL=z
ENV CARGO_PROFILE_RELEASE_STRIP=true

RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -m -u 1000 worm

WORKDIR /app

COPY --from=builder /app/target/release/worm .
COPY --from=builder /app/config.json /app/system-prompt.txt ./

RUN chown -R worm:worm /app

USER worm


CMD ["./worm"]

