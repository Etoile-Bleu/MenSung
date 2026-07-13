# Development commands for MenSung. Run `make help` to list them. Every
# target mirrors a step from CONTRIBUTING.md and .github/workflows/ci.yml,
# so `make ci` is exactly what the pipeline runs, locally, before you push.

.DEFAULT_GOAL := help
.PHONY: help build release test fmt fmt-check clippy lint audit deny \
        check ci clean run cli fuzz install-tools

CARGO := cargo

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*##' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*##"}; {printf "\033[36m%-15s\033[0m %s\n", $$1, $$2}'

build: ## Build the workspace in debug mode
	$(CARGO) build --workspace

release: ## Build the release binary (target/release/mensung)
	$(CARGO) build --release -p mensung-client

test: ## Run the full workspace test suite
	$(CARGO) test --workspace

fmt: ## Format the workspace in place
	$(CARGO) fmt

fmt-check: ## Check formatting without modifying files
	$(CARGO) fmt --check

clippy: ## Run clippy with warnings denied, matching CI
	$(CARGO) clippy --workspace --all-targets -- -D warnings

lint: fmt-check clippy ## Run fmt-check and clippy together

audit: ## Check dependencies for known security advisories (needs cargo-audit)
	$(CARGO) audit

deny: ## Check dependency licenses and bans (needs cargo-deny)
	$(CARGO) deny check

check: lint test ## Fast local pre-push check: lint plus test

ci: fmt-check clippy test audit deny ## Run everything CI runs, in the same order

clean: ## Remove build artifacts
	$(CARGO) clean

run: ## Launch the interactive TUI
	$(CARGO) run -p mensung-client

cli: ## Run the CLI; usage: make cli ARGS="Aspirin Warfarin"
	$(CARGO) run -p mensung-client -- $(ARGS)

fuzz: ## Run the mensung-db fuzz target (needs nightly and cargo-fuzz)
	cd fuzz && $(CARGO) +nightly fuzz run parse_men

install-tools: ## Install cargo-audit, cargo-deny, and cargo-fuzz
	$(CARGO) install cargo-audit cargo-deny cargo-fuzz
