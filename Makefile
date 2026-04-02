.PHONY: setup dev-server docker-build docker-run clean format lint

setup:
	cd web && bun install && bun run build
	cargo build

dev-server:
	cd web && bun run build
	cargo run -p slasha-server

dev-cli:
	cargo run -p slasha-cli -- $(ARGS)

docker-build:
	docker build -t slasha-server:latest .

docker-run:
	docker run -p 3000:3000 slasha-server:latest

clean:
	@cargo clean
	@rm -rf web/build

format:
	@cargo fmt
	@cd web && bun run format

lint:
	@cargo clippy
	@cd web && bun run lint