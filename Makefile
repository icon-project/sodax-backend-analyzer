SHELL := /bin/bash
.PHONY: help

help:
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-15s\033[0m %s\n", $$1, $$2}'

clean: ## Clean the project using cargo
	cargo clean

build: ## Build the project using cargo
	cargo build

lint: ## Lint the project using cargo
	@rustup component add clippy 2> /dev/null
	cargo clippy

fmt: ## Format the project using cargo
	@rustup component add rustfmt 2> /dev/null
	cargo fmt

test: ## Run tests using cargo
	cargo test

doc: ## Generate documentation using cargo
	cargo doc --no-deps --open

run-example: ## Run an example with the specified argument (usage: make run-example-1)
	@echo "Usage: make run-example-<number>"
	@echo "Example: make run-example-1"
	@echo "Available examples: example-1, example-2, example-3, example-4, example-5, example-6, example-7, example-8"

run-example-%: ## Run a specific example (usage: make run-example-1)
	@echo "Running example: example-$*"
	cargo run --bin example-$*

# This allows Make to accept any target name as an argument
%:
	@:
