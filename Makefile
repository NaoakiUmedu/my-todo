build:
	docker-compose build
db-start:
	docker-compose up -d
db-bash:
	docker compose exec database bash
db-down:
	docker compose down
dev:
	cargo watch -x run
test:
	cargo test
