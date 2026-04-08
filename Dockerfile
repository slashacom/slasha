FROM oven/bun:latest AS frontend-builder
WORKDIR /app/web
COPY web/package.json web/bun.lock ./
RUN bun install
COPY web/ .
RUN NODE_ENV=production bun run build

FROM lukemathwalker/cargo-chef:latest-rust-slim-bookworm AS chef
RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev libsqlite3-dev && \
    rm -rf /var/lib/apt/lists/*
WORKDIR /app

FROM chef AS planner
COPY Cargo.toml Cargo.lock ./
COPY crates/ ./crates/
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY Cargo.toml Cargo.lock ./
COPY crates/ ./crates/
COPY --from=frontend-builder /app/web/build/client /app/web/build/client
RUN cargo build --release -p slasha-server --features bundle \
 && cargo build --release -p slasha-git-ssh-handler

FROM debian:bookworm-slim AS runtime
RUN apt-get update && \
    apt-get install -y openssh-server git libssl3 ca-certificates libsqlite3-0 && \
    rm -rf /var/lib/apt/lists/*

RUN mkdir /var/run/sshd \
 && useradd -m slasha \
 && mkdir -p /home/slasha/.ssh \
 && chmod 700 /home/slasha/.ssh \
 && chown -R slasha:slasha /home/slasha/.ssh

COPY --from=builder /app/target/release/slasha-server /usr/local/bin/slasha-server
COPY --from=builder /app/target/release/slasha-git-ssh-handler /usr/local/bin/slasha-git-ssh-handler
RUN chmod +x /usr/local/bin/slasha-server /usr/local/bin/slasha-git-ssh-handler

EXPOSE 3000 22
CMD ["/bin/bash", "-c", "/usr/sbin/sshd -D & exec su -s /bin/bash slasha -c 'exec slasha-server'"]