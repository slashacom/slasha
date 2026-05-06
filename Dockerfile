FROM oven/bun:latest AS frontend-builder
WORKDIR /app/web
COPY web/package.json web/bun.lock ./
RUN bun install
COPY web/ .
RUN NODE_ENV=production bun run build

FROM lukemathwalker/cargo-chef:latest-rust-slim-bookworm AS chef
# libdbus-1-dev: keyring crate's `sync-secret-service` feature pulls in
# libdbus-sys on Linux. The keyring is only used by the CLI on user
# laptops (it has no working DBus session inside the container), but
# the dep still has to compile when we build the bundled binary.
RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev libsqlite3-dev libdbus-1-dev && \
    rm -rf /var/lib/apt/lists/*
WORKDIR /app

FROM chef AS planner
COPY Cargo.toml Cargo.lock ./
COPY slasha-server/ ./slasha-server/
COPY slasha-cli/ ./slasha-cli/
COPY slasha-db/ ./slasha-db/
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY Cargo.toml Cargo.lock ./
COPY slasha-server/ ./slasha-server/
COPY slasha-cli/ ./slasha-cli/
COPY slasha-db/ ./slasha-db/
COPY --from=frontend-builder /app/web/build/client /app/web/build/client
RUN cargo build --release -p slasha-cli --features serve-bundle

FROM debian:bookworm-slim AS runtime
# docker-ce-cli + docker-buildx-plugin are required: slasha-server shells out
# to `docker buildx build` for both Dockerfile and Railpack build paths
# (talks to the host daemon via the bind-mounted /var/run/docker.sock).
RUN apt-get update && \
    apt-get install -y openssh-server git libssl3 ca-certificates libsqlite3-0 curl gnupg && \
    install -m 0755 -d /etc/apt/keyrings && \
    curl -fsSL https://download.docker.com/linux/debian/gpg | gpg --dearmor -o /etc/apt/keyrings/docker.gpg && \
    chmod a+r /etc/apt/keyrings/docker.gpg && \
    echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.gpg] https://download.docker.com/linux/debian bookworm stable" > /etc/apt/sources.list.d/docker.list && \
    apt-get update && \
    apt-get install -y docker-ce-cli docker-buildx-plugin && \
    rm -rf /var/lib/apt/lists/*

RUN mkdir /var/run/sshd \
 && useradd -m slasha \
 && install -d -m 0700 -o slasha -g slasha /home/slasha/.ssh \
 && install -d -m 0755 -o slasha -g slasha /home/slasha/.slasha \
 && touch /home/slasha/.ssh/.keep /home/slasha/.slasha/.keep \
 && chown slasha:slasha /home/slasha/.ssh/.keep /home/slasha/.slasha/.keep \
 && rm -f /etc/ssh/ssh_host_*

COPY --from=builder /app/target/release/slasha /usr/local/bin/slasha
COPY docker-entrypoint.sh /usr/local/bin/docker-entrypoint.sh
RUN chmod +x /usr/local/bin/slasha /usr/local/bin/docker-entrypoint.sh

EXPOSE 3000 2222
ENTRYPOINT ["/usr/local/bin/docker-entrypoint.sh"]