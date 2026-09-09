-include .env
export

DC   = docker compose -f compose.dev.yml
DCP  = docker compose -f compose.prod.yml
SVC  = api
FE   = frontend

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-20s\033[0m %s\n", $$1, $$2}'

up: ## Start API + frontend in dev mode (hot reload)
	$(DC) up

down: ## Stop all containers
	$(DC) down

prod-up: ## Build + start the production stack (postgres + api + nginx)
	$(DCP) up -d --build

prod-down: ## Stop the production stack
	$(DCP) down

prod-logs: ## Tail production logs
	$(DCP) logs -f

build: ## Rebuild Docker images
	$(DC) build

shell: ## Shell inside api container
	$(DC) exec $(SVC) sh

shell-fe: ## Shell inside frontend container
	$(DC) exec $(FE) sh

check: ## cargo check
	$(DC) exec $(SVC) cargo check

clippy: ## cargo clippy -D warnings
	$(DC) exec $(SVC) cargo clippy --all-targets -- -D warnings

fmt: ## cargo fmt
	$(DC) exec $(SVC) cargo fmt

test: ## Run the Rust test suite (unit + integration) in the container
	$(DC) exec $(SVC) cargo test

test-local: ## Run the Rust test suite on the host
	cargo test

ci: ## fmt check + clippy + tests (host)
	cargo fmt --check
	cargo clippy --all-targets -- -D warnings
	cargo test

tsc: ## TypeScript check (frontend)
	$(DC) exec $(FE) pnpm tsc --noEmit

lint: ## ESLint (frontend)
	$(DC) exec $(FE) pnpm lint

prune: ## Delete completed/failed jobs older than DAYS=7 (add DRY=1 to preview)
	$(DC) exec $(SVC) cargo run --quiet --bin server -- jobs prune --days $(or $(DAYS),7) $(if $(DRY),--dry-run,)

check-config: ## Print the effective config + run the startup checks
	$(DC) exec $(SVC) cargo run --quiet --bin server -- check

logs: ## Tail API logs
	$(DC) logs -f $(SVC)

logs-fe: ## Tail frontend logs
	$(DC) logs -f $(FE)
