-include .env
export

DC  = docker compose -f compose.dev.yml
SVC = api

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-20s\033[0m %s\n", $$1, $$2}'

up: ## Start the API in dev mode (hot reload)
	$(DC) up

down: ## Stop all containers
	$(DC) down

build: ## Rebuild Docker image
	$(DC) build

shell: ## Shell inside api container
	$(DC) exec $(SVC) sh

check: ## cargo check
	$(DC) exec $(SVC) cargo check

clippy: ## cargo clippy -D warnings
	$(DC) exec $(SVC) cargo clippy -- -D warnings

fmt: ## cargo fmt
	$(DC) exec $(SVC) cargo fmt

test: ## cargo nextest run
	$(DC) exec $(SVC) cargo nextest run

logs: ## Tail API logs
	$(DC) logs -f $(SVC)
