test:
	cargo fmt --all
	cargo clippy --all-targets -- -D warnings
	cargo test --all
	pytest -q || true      # placeholder until tests exist

build:
	export DOCKER_BUILDKIT=1
	docker compose build

up:
	docker compose up -d

down:
	docker compose down

logs:
	docker compose logs -f

clean:
	docker compose down -v
	docker system prune -f 