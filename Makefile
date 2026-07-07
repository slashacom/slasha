.PHONY: dev dev-cli dev-bundle format lint test gen-models docker-up docker-logs clean

.DEFAULT_GOAL := dev

dev:
	@cd web && bun install
	@test -f .env || cp .env.example .env
	@trap 'kill $$(jobs -p)' EXIT; \
	cargo run -p slasha-cli --no-default-features --features serve,vendored -- serve & \
	cd web && bun run dev & \
	wait

dev-cli:
	cargo run -p slasha-cli --no-default-features --features serve,vendored -- $(ARGS)

dev-bundle:
	@cd web && bun install
	@echo "Building frontend..."
	@cd web && bun run build
	@echo "Running bundled server..."
	cargo run -p slasha-cli -- serve

format:
	@cargo +nightly fmt --all
	@cd web && bun run format

lint:
	@cargo clippy --workspace --all-targets
	@cd web && bun run lint

test:
	@cargo test --workspace

gen-models:
	@echo "Generating TS models..."
	@cargo test -p slasha-db
	@echo "Done."

docker-up:
	docker compose -f docker/docker-compose.yml up --build -d

docker-logs:
	docker compose -f docker/docker-compose.yml logs -f

clean:
	@echo "Cleaning workspace..."
	@cargo clean
	@rm -rf web/build web/node_modules
	@echo "Done."