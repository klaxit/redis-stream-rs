.PHONY: $(MAKECMDGOALS) help
.DEFAULT_GOAL := help

FMT_TITLE='\\033[1m'
FMT_PRIMARY='\\033[36m'
FMT_END='\\033[0m'
help:
	@awk 'BEGIN {FS = ":.*##"; printf "Usage: make $(FMT_PRIMARY)<target>$(FMT_END)\n"} \
	/^[a-zA-Z0-9_-]+:.*?##/ { printf "  $(FMT_PRIMARY)%-46s$(FMT_END) %s\n", $$1, $$2 } \
	/^##@/ { printf "\n$(FMT_TITLE)%s$(FMT_END)\n", substr($$0, 5) } \
	' $(MAKEFILE_LIST)

all: check test ## Run all tests and checks

##@ Testing

test: test-unit ## Test everything

test-unit: ## Run the unit test suite (require Redis)
	cargo test -- --nocapture

##@ Checking

check: check-code check-clippy check-fmt ## Check everything

check-code: ## Check code
	cargo check --all-features

check-clippy: ## Check code with Clippy
	@rustup component add clippy 2> /dev/null
	cargo clippy --all-targets --all-features -- -D warnings

check-fmt: ## Check that all files are formatted properly
	@rustup component add rustfmt 2> /dev/null
	cargo fmt -- --check

##@ Utility

clean: ## Clean everything
	cargo clean

doc: ## Generate documentation
	cargo doc --all-features --no-deps

fmt: ## Format code
	cargo fmt
