db-build:
	docker-compose build
db:
	docker-compose up -d
db-bash:
	docker compose exec database bash
db-stop:
	docker compose stop
dev:
	sqlx db create
	sqlx migrate run
	cargo watch -x run
clean:
	cargo clean
test:
	cargo test
# standalone test
test-s:
	cargo test --no-default-features

