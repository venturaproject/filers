-include .env
export

DC   = docker compose -f compose.dev.yml
SVC  = api
FE   = frontend

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-20s\033[0m %s\n", $$1, $$2}'

up: ## Start API + frontend in dev mode (hot reload)
	$(DC) up

down: ## Stop all containers
	$(DC) down

build: ## Rebuild Docker images
	$(DC) build

shell: ## Shell inside api container
	$(DC) exec $(SVC) sh

shell-fe: ## Shell inside frontend container
	$(DC) exec $(FE) sh

check: ## cargo check
	$(DC) exec $(SVC) cargo check

clippy: ## cargo clippy -D warnings
	$(DC) exec $(SVC) cargo clippy -- -D warnings

fmt: ## cargo fmt
	$(DC) exec $(SVC) cargo fmt

test: ## cargo nextest run
	$(DC) exec $(SVC) cargo nextest run

tsc: ## TypeScript check (frontend)
	$(DC) exec $(FE) pnpm tsc --noEmit

lint: ## ESLint (frontend)
	$(DC) exec $(FE) pnpm lint

logs: ## Tail API logs
	$(DC) logs -f $(SVC)

logs-fe: ## Tail frontend logs
	$(DC) logs -f $(FE)
