FROM rust:1.82-bookworm AS builder
WORKDIR /app

# 依赖缓存层
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && cargo build --release && rm -rf src

# 真实源码
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/youlai-axum /app/youlai-axum
COPY --from=builder /app/config /app/config
COPY --from=builder /app/templates /app/templates
EXPOSE 8000
CMD ["/app/youlai-axum"]
