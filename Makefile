# LlamaDesk developer tasks. `make help` lists everything.
.DEFAULT_GOAL := help
SHELL := /bin/bash

.PHONY: help build install install-user install-system uninstall run dev \
        test test-frontend test-backend lint fmt check clean

help: ## Show this help
	@printf 'LlamaDesk make targets:\n\n'
	@grep -hE '^[a-zA-Z_-]+:.*?## ' $(MAKEFILE_LIST) | sort \
		| awk 'BEGIN {FS = ":.*?## "} {printf "  \033[36m%-16s\033[0m %s\n", $$1, $$2}'

build: ## Build the frontend and application bundles (scripts/build.sh)
	scripts/build.sh

install: install-user ## Install for the current user (alias of install-user)

install-user: ## Install under ~/.local without sudo
	scripts/install.sh --user

install-system: ## Install system-wide (/opt/llamadesk + /usr/local/bin, sudo)
	scripts/install.sh --system

uninstall: ## Remove a user install (pass ARGS="--purge" to delete data)
	scripts/uninstall.sh --user $(ARGS)

run: ## Run the built release binary
	./target/release/llamadesk

dev: ## Run the desktop app with hot reload
	cargo tauri dev

test: test-backend test-frontend ## Run backend and frontend tests

# `npm run test:run` is provided by the frontend (package.json). If the script
# is not present yet, this target will fail until the frontend work lands.
test-frontend: ## Run frontend tests (vitest run)
	npm run test:run

test-backend: ## Run Rust tests
	cargo test --manifest-path src-tauri/Cargo.toml

lint: ## Run clippy (deny warnings) and the frontend build/typecheck
	cargo clippy --all-targets -- -D warnings
	npm run build

fmt: ## Format Rust sources
	cargo fmt --all

check: ## Fast compile + typecheck without producing bundles
	cargo check --manifest-path src-tauri/Cargo.toml
	npm run build

clean: ## Remove build output (cargo target + dist/)
	cargo clean
	rm -rf dist
