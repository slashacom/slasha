.PHONY: setup gen-models dev dev-cli dev-bundle docker-up docker-logs clean format lint

.DEFAULT_GOAL := dev

setup:
	cd web && bun install
	cargo build

gen-models:
	cd crates/models && cargo test

dev: setup
	@test -f .env || cp .env.example .env
	@trap 'kill $$(jobs -p)' EXIT; \
	cargo run -p slasha-server & \
	cd web && bun run dev & \
	wait

dev-cli:
	cargo run -p slasha-cli -- $(ARGS)

dev-bundle:
	cd web && bun run build
	cargo run -p slasha-server --features bundle

docker-up:
	docker compose up --build

docker-logs:
	docker compose logs -f

clean:
	@cargo clean
	@rm -rf web/build
	@rm -rf db/
	@rm -rf repos/

format:
	@cargo fmt
	@cd web && bun run format

lint:
	@cargo clippy
	@cd web && bun run lint