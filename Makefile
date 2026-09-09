.PHONY: dev dev-cli format lint test gen-models docker-up docker-logs clean

.DEFAULT_GOAL := dev

FEATURES ?= serve

BREW_PREFIX := $(shell brew --prefix 2>/dev/null)
ifneq ($(BREW_PREFIX),)
export LIBRARY_PATH := $(BREW_PREFIX)/lib:$(LIBRARY_PATH)
endif

dev:
	@cd web && bun install
	@test -f .env || cp .env.example .env
	@trap 'kill $$(jobs -p)' EXIT; \
	cargo run -p slasha-cli --no-default-features --features $(FEATURES) -- serve & \
	cd web && bun run dev & \
	wait

dev-cli:
	cargo run -p slasha-cli --no-default-features --features $(FEATURES) -- $(ARGS)

format:
	@cargo +nightly fmt --all
	@cd web && bun run format

lint:
	@cargo clippy --workspace --all-targets --no-default-features --features serve

test:
	@cargo test --workspace --no-default-features --features serve

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