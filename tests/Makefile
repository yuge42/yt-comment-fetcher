.PHONY: help docker-test docker-build docker-clean test clean

help: ## Show this help message
	@echo "Available targets:"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2}'

docker-test: ## Run tests using Docker Compose
	docker compose up --build --abort-on-container-exit --exit-code-from tests
	docker compose down

docker-build: ## Build Docker images
	docker compose build

docker-clean: ## Clean up Docker resources
	docker compose down -v
	docker system prune -f

test: ## Run tests locally (requires Gauge and npm to be installed, and fetcher binary)
	npm test

clean: ## Clean local test artifacts
	rm -rf reports logs .gauge node_modules proto-gen
