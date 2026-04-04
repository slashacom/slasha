.PHONY: setup migrate gen-models dev dev-cli dev-bundle docker-build docker-run clean format lint

.DEFAULT_GOAL := dev

setup: migrate
	cd web && bun install && bun run build
	cargo build

migrate:
	mkdir -p db && touch db/slasha.db
	cd crates/server && diesel migration run --database-url ../../db/slasha.db

gen-models:
	cd crates/models && cargo test

dev:
	@trap 'kill $$(jobs -p)' EXIT; \
	cargo run -p slasha-server & \
	cd web && bun run dev & \
	wait

dev-cli:
	cargo run -p slasha-cli -- $(ARGS)

dev-bundle:
	cd web && bun run build
	cargo run -p slasha-server --features bundle

docker-build:
	docker build -t slasha-server:latest .

docker-run:
	docker run --rm --init -p 3000:3000 slasha-server:latest

clean:
	@cargo clean
	@rm -rf web/build

format:
	@cargo fmt
	@cd web && bun run format

lint:
	@cargo clippy
	@cd web && bun run lint