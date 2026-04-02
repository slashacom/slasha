FROM oven/bun:latest AS frontend-builder
WORKDIR /app/web
COPY web/package.json web/bun.lock ./
RUN bun install
COPY web/ .
RUN NODE_ENV=production bun run build

FROM rust:latest AS backend-builder
WORKDIR /app
COPY crates/ ./crates/
COPY Cargo.toml Cargo.lock ./
COPY --from=frontend-builder /app/web/build/client /app/web/build/client

RUN cargo build --release -p slasha-server

FROM debian:bookworm-slim
WORKDIR /app
COPY --from=backend-builder /app/target/release/slasha-server /usr/local/bin/slasha-server

RUN apt-get update && apt-get install -y libssl-dev ca-certificates && rm -rf /var/lib/apt/lists/*

EXPOSE 3000
CMD ["slasha-server"]
