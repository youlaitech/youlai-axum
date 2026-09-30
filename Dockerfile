FROM rust:1.83-slim AS builder

WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/youlai-axum /usr/local/bin/youlai-axum
COPY --from=builder /app/config /app/config

EXPOSE 8000
CMD ["youlai-axum"]
