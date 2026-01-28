FROM rustlang/rust:nightly-bookworm AS builder

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release || true
RUN rm -rf src

COPY .sqlx ./.sqlx

COPY migrations ./migrations

COPY src ./src

COPY system-prompt.txt gemini_prompt.txt ./

ENV SQLX_OFFLINE=true
RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -m -u 1000 worm

WORKDIR /app

COPY --from=builder /app/target/release/worm .
COPY --from=builder /app/system-prompt.txt .
COPY --from=builder /app/gemini_prompt.txt .

RUN mkdir -p /app/data && chown -R worm:worm /app
USER worm

CMD ["./worm"]
