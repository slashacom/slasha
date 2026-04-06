FROM oven/bun:latest AS frontend-builder
WORKDIR /app/web
COPY web/package.json web/bun.lock ./
RUN bun install
COPY web/ .
RUN NODE_ENV=production bun run build

FROM debian:bookworm-slim AS backend-builder
WORKDIR /app
RUN apt-get update && apt-get install -y \
    curl build-essential libsqlite3-dev pkg-config libssl-dev ca-certificates \
    && rm -rf /var/lib/apt/lists/*
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y --default-toolchain stable
ENV PATH="/root/.cargo/bin:${PATH}"
COPY Cargo.toml Cargo.lock ./
COPY crates/ ./crates/
COPY --from=frontend-builder /app/web/build/client /app/web/build/client
RUN cargo build --release -p slasha-server --features bundle

FROM debian:bookworm-slim AS runtime
WORKDIR /app
RUN apt-get update && apt-get install -y \
    libssl3 ca-certificates git libsqlite3-0 \
    && rm -rf /var/lib/apt/lists/*
COPY --from=backend-builder /app/target/release/slasha-server /usr/local/bin/slasha-server
EXPOSE 3000
CMD ["slasha-server"]