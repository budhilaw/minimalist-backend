# Portfolio Backend Makefile
# Run 'make help' to see all available commands

.PHONY: help setup clean build run test check lint format deps update migrate dev prod docker-build docker-run docker-stop

# Default target
.DEFAULT_GOAL := help

# Colors for output
RED := \033[31m
GREEN := \033[32m
YELLOW := \033[33m
BLUE := \033[34m
CYAN := \033[36m
RESET := \033[0m

##@ Setup Commands

setup: ## 🚀 Initial project setup (copy config files, install deps)
	@echo "$(CYAN)🚀 Setting up Portfolio Backend...$(RESET)"
	@if [ ! -f ".config.yaml" ]; then \
		cp example.config.yaml .config.yaml && \
		echo "$(GREEN)✅ Created .config.yaml from template$(RESET)"; \
	else \
		echo "$(YELLOW)⚠️  .config.yaml already exists$(RESET)"; \
	fi
	@if [ ! -f ".secret.yaml" ]; then \
		cp example.secret.yaml .secret.yaml && \
		chmod 600 .secret.yaml && \
		echo "$(GREEN)✅ Created .secret.yaml from template$(RESET)" && \
		echo "$(RED)⚠️  IMPORTANT: Edit .secret.yaml with your actual credentials!$(RESET)"; \
	else \
		echo "$(YELLOW)⚠️  .secret.yaml already exists$(RESET)"; \
	fi
	@$(MAKE) deps
	@echo "$(GREEN)🎉 Setup complete! Run 'make dev' to start development server$(RESET)"

deps: ## 📦 Install Rust dependencies
	@echo "$(CYAN)📦 Installing dependencies...$(RESET)"
	@cargo fetch

##@ Development Commands

dev: ## 🔧 Start development server with hot reload
	@echo "$(CYAN)🔧 Starting development server...$(RESET)"
	@cargo run

build: ## 🏗️  Build the project in debug mode
	@echo "$(CYAN)🏗️  Building project...$(RESET)"
	@cargo build

build-release: ## 🏗️  Build the project in release mode
	@echo "$(CYAN)🏗️  Building project (release)...$(RESET)"
	@cargo build --release

run: ## ▶️  Run the compiled binary
	@echo "$(CYAN)▶️  Running application...$(RESET)"
	@cargo run --release

watch: ## 👀 Watch for changes and rebuild automatically
	@echo "$(CYAN)👀 Watching for changes...$(RESET)"
	@cargo watch -x run

##@ Testing Commands

test: ## 🧪 Run all tests
	@echo "$(CYAN)🧪 Running tests...$(RESET)"
	@cargo test

test-verbose: ## 🧪 Run tests with verbose output
	@echo "$(CYAN)🧪 Running tests (verbose)...$(RESET)"
	@cargo test -- --nocapture

test-coverage: ## 📊 Generate test coverage report
	@echo "$(CYAN)📊 Generating coverage report...$(RESET)"
	@cargo tarpaulin --out Html

bench: ## ⚡ Run benchmarks
	@echo "$(CYAN)⚡ Running benchmarks...$(RESET)"
	@cargo bench

##@ Code Quality Commands

check: ## 🔍 Check code for errors without building
	@echo "$(CYAN)🔍 Checking code...$(RESET)"
	@cargo check

lint: ## 🔧 Run clippy linter
	@echo "$(CYAN)🔧 Running linter...$(RESET)"
	@cargo clippy -- -D warnings

lint-fix: ## 🔧 Run clippy and fix issues automatically
	@echo "$(CYAN)🔧 Running linter with auto-fix...$(RESET)"
	@cargo clippy --fix --allow-dirty -- -D warnings

format: ## ✨ Format code with rustfmt
	@echo "$(CYAN)✨ Formatting code...$(RESET)"
	@cargo fmt

format-check: ## ✨ Check if code is formatted correctly
	@echo "$(CYAN)✨ Checking code formatting...$(RESET)"
	@cargo fmt -- --check

audit: ## 🔒 Check for security vulnerabilities
	@echo "$(CYAN)🔒 Auditing dependencies...$(RESET)"
	@cargo audit

doc: ## 📚 Generate documentation
	@echo "$(CYAN)📚 Generating documentation...$(RESET)"
	@cargo doc --open

##@ Database Commands

db-create: ## 🗄️  Create database
	@echo "$(CYAN)🗄️  Creating database...$(RESET)"
	@createdb portfolio_db || echo "$(YELLOW)Database might already exist$(RESET)"

db-drop: ## 🗑️  Drop database
	@echo "$(RED)🗑️  Dropping database...$(RESET)"
	@dropdb portfolio_db || echo "$(YELLOW)Database might not exist$(RESET)"

db-reset: db-drop db-create ## 🔄 Reset database (drop and recreate)
	@echo "$(GREEN)🔄 Database reset complete$(RESET)"

migrate: ## 🚀 Run database migrations
	@echo "$(CYAN)🚀 Running migrations...$(RESET)"
	@sqlx migrate run

migrate-revert: ## ⏪ Revert last migration
	@echo "$(CYAN)⏪ Reverting last migration...$(RESET)"
	@sqlx migrate revert

sqlx-prepare: ## 🔧 Generate SQLx query cache for offline compilation
	@echo "$(CYAN)🔧 Generating SQLx query cache...$(RESET)"
	@cargo sqlx prepare

seed: ## 🌱 Seed database with dummy data
	@echo "$(CYAN)🌱 Seeding database with dummy data...$(RESET)"
	@cargo run --release seed

seed-reset: db-reset migrate seed ## 🔄 Reset database and seed with fresh data
	@echo "$(GREEN)🔄 Database reset and seeding completed!$(RESET)"

##@ Docker Commands

docker-build: ## 🐳 Build Docker image
	@echo "$(CYAN)🐳 Building Docker image...$(RESET)"
	@docker build -t portfolio-backend .

docker-build-no-cache: ## 🐳 Build Docker image without cache
	@echo "$(CYAN)🐳 Building Docker image (no cache)...$(RESET)"
	@docker build --no-cache -t portfolio-backend .

docker-run: ## 🐳 Run Docker container
	@echo "$(CYAN)🐳 Running Docker container...$(RESET)"
	@docker run -p 8000:8000 --env-file .env portfolio-backend

docker-run-dev: ## 🐳 Run Docker container with development settings
	@echo "$(CYAN)🐳 Running Docker container (development)...$(RESET)"
	@docker run -p 8000:8000 -e RUST_LOG=debug --env-file .env portfolio-backend

docker-compose-up: ## 🐳 Start all services with docker-compose (production)
	@echo "$(CYAN)🐳 Starting services with docker-compose...$(RESET)"
	@docker-compose -f docker-compose.prod.yml up -d

docker-compose-dev: ## 🐳 Start development services with docker-compose
	@echo "$(CYAN)🐳 Starting development services...$(RESET)"
	@docker-compose -f docker-compose.dev.yml up -d --build

docker-compose-up-build: ## 🐳 Start services and rebuild images
	@echo "$(CYAN)🐳 Starting services with rebuild...$(RESET)"
	@docker-compose -f docker-compose.prod.yml up -d --build

docker-compose-down: ## 🐳 Stop all services with docker-compose
	@echo "$(CYAN)🐳 Stopping services with docker-compose...$(RESET)"
	@docker-compose -f docker-compose.prod.yml down

docker-compose-down-volumes: ## 🐳 Stop services and remove volumes
	@echo "$(CYAN)🐳 Stopping services and removing volumes...$(RESET)"
	@docker-compose -f docker-compose.prod.yml down -v

docker-logs: ## 🐳 View Docker container logs
	@echo "$(CYAN)🐳 Viewing Docker logs...$(RESET)"
	@docker-compose -f docker-compose.prod.yml logs -f

docker-logs-api: ## 🐳 View API container logs only
	@echo "$(CYAN)🐳 Viewing API logs...$(RESET)"
	@docker-compose -f docker-compose.prod.yml logs -f api

docker-migrate: ## 🐳 Run database migrations in Docker
	@echo "$(CYAN)🐳 Running migrations in Docker...$(RESET)"
	@docker-compose -f docker-compose.prod.yml run --rm migrate

docker-seed: ## 🐳 Seed database in Docker
	@echo "$(CYAN)🐳 Seeding database in Docker...$(RESET)"
	@docker-compose -f docker-compose.prod.yml run --rm api ./seed

docker-shell: ## 🐳 Open shell in running API container
	@echo "$(CYAN)🐳 Opening shell in API container...$(RESET)"
	@docker-compose -f docker-compose.prod.yml exec api sh

docker-clean: ## 🐳 Clean Docker images and containers
	@echo "$(CYAN)🐳 Cleaning Docker resources...$(RESET)"
	@docker system prune -f
	@docker image prune -f

docker-reset: docker-compose-down-volumes docker-clean docker-compose-up-build ## 🐳 Complete Docker reset
	@echo "$(GREEN)🐳 Docker environment reset complete!$(RESET)"

##@ Maintenance Commands

clean: ## 🧹 Clean build artifacts
	@echo "$(CYAN)🧹 Cleaning build artifacts...$(RESET)"
	@cargo clean

update: ## 📈 Update all dependencies
	@echo "$(CYAN)📈 Updating dependencies...$(RESET)"
	@cargo update

outdated: ## 📋 Check for outdated dependencies
	@echo "$(CYAN)📋 Checking for outdated dependencies...$(RESET)"
	@cargo outdated

tree: ## 🌳 Show dependency tree
	@echo "$(CYAN)🌳 Showing dependency tree...$(RESET)"
	@cargo tree

##@ Utility Commands

env-check: ## ✅ Check environment and dependencies
	@echo "$(CYAN)✅ Checking environment...$(RESET)"
	@echo "Rust version: $(shell rustc --version)"
	@echo "Cargo version: $(shell cargo --version)"
	@echo "PostgreSQL: $(shell psql --version 2>/dev/null || echo 'Not installed')"
	@echo "Redis: $(shell redis-cli --version 2>/dev/null || echo 'Not installed')"
	@echo "Docker: $(shell docker --version 2>/dev/null || echo 'Not installed')"

logs: ## 📋 View application logs
	@echo "$(CYAN)📋 Viewing logs...$(RESET)"
	@tail -f logs/app.log 2>/dev/null || echo "$(YELLOW)No log file found$(RESET)"

config-validate: ## ✅ Validate configuration files
	@echo "$(CYAN)✅ Validating configuration...$(RESET)"
	@if [ -f ".config.yaml" ]; then \
		echo "$(GREEN).config.yaml exists$(RESET)"; \
	else \
		echo "$(RED).config.yaml missing! Run 'make setup'$(RESET)"; \
	fi
	@if [ -f ".secret.yaml" ]; then \
		echo "$(GREEN).secret.yaml exists$(RESET)"; \
	else \
		echo "$(RED).secret.yaml missing! Run 'make setup'$(RESET)"; \
	fi

##@ CI/CD Commands

ci: check lint test ## 🔄 Run CI pipeline (check, lint, test)
	@echo "$(GREEN)🔄 CI pipeline completed successfully!$(RESET)"

pre-commit: format lint test ## 🚀 Run pre-commit checks
	@echo "$(GREEN)🚀 Pre-commit checks passed!$(RESET)"

release-check: build-release test audit ## 📦 Pre-release validation
	@echo "$(GREEN)📦 Release checks completed!$(RESET)"

##@ Help

help: ## 📖 Show this help message
	@echo "$(CYAN)Portfolio Backend - Available Commands$(RESET)"
	@echo ""
	@awk 'BEGIN {FS = ":.*##"; printf "Usage: make $(CYAN)<target>$(RESET)\n\n"} \
		/^[a-zA-Z_0-9-]+:.*?##/ { printf "  $(CYAN)%-20s$(RESET) %s\n", $$1, $$2 } \
		/^##@/ { printf "\n$(YELLOW)%s$(RESET)\n", substr($$0, 5) } ' $(MAKEFILE_LIST)
	@echo ""
	@echo "$(YELLOW)Examples:$(RESET)"
	@echo "  make setup          # Initial project setup"
	@echo "  make dev           # Start development server"
	@echo "  make test          # Run all tests"
	@echo "  make ci            # Run full CI pipeline"
	@echo "  make docker-build  # Build Docker image" 