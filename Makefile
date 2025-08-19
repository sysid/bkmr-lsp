.DEFAULT_GOAL := help
#MAKEFLAGS += --no-print-directory

# You can set these variables from the command line, and also from the environment for the first two.
PREFIX ?= /usr/local
BINPREFIX ?= "$(PREFIX)/bin"

VERSION       = $(shell cat VERSION)

SHELL	= bash
.ONESHELL:

app_root := $(if $(PROJ_DIR),$(PROJ_DIR),$(CURDIR))

pkg_src =  $(app_root)/bkmr-lsp
tests_src = $(app_root)/bkmr-lsp/tests
BINARY = bkmr-lsp

# Makefile directory
CODE_DIR := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))

# define files
MANS = $(wildcard ./*.md)
MAN_HTML = $(MANS:.md=.html)
MAN_PAGES = $(MANS:.md=.1)
# avoid circular targets
MAN_BINS = $(filter-out ./tw-extras.md, $(MANS))

################################################################################
# Admin \
ADMIN::  ## ##################################################################

.PHONY: init
init:  ## init
	@rm -fv ~/bkmr-lsp.log
	@rm -fv $(HOME)/dev/s/public/bkmr-intellij-plugin/build/idea-sandbox/IU-2025.1.1.1/log/idea.log

.PHONY: test
test:  ## tests, single-threaded
	pushd $(pkg_src) && cargo test -- --test-threads=1

.PHONY: log-plugin
log-plugin:  ## log-plugin
	tail -f $(HOME)/dev/s/public/bkmr-intellij-plugin/build/idea-sandbox/IU-2025.1.1.1/log/idea.log | grep -u completion

.PHONY: log-lsp
log-lsp:  ## log-lsp
	tail -f ~/bkmr-lsp.log | sed -u 's/^[^[{]*:[[:space:]]*//' | jq -C -c .
	#tail -f ~/bkmr-lsp.log | sed -u 's/^[^[{]*:[[:space:]]*//' | jq -C .

################################################################################
# Building, Deploying \
BUILDING:  ## ##################################################################

.PHONY: all
all: clean build install  ## all
	:

.PHONY: all-fast
all-fast: build-fast install-debug  ## all-fast: no release build
	:

.PHONY: upload
upload:  ## upload
	@if [ -z "$$CARGO_REGISTRY_TOKEN" ]; then \
		echo "Error: CARGO_REGISTRY_TOKEN is not set"; \
		exit 1; \
	fi
	@echo "CARGO_REGISTRY_TOKEN is set"
	pushd $(pkg_src) && cargo release publish --execute

.PHONY: build
build:  ## build
	pushd $(pkg_src) && cargo build --release

.PHONY: build-fast
build-fast:  ## build-fast
	pushd $(pkg_src) && cargo build

.PHONY: install-debug
install-debug: uninstall  ## install-debug (links to target/debug)
	@ln -vsf $(realpath bkmr-lsp/target/debug/$(BINARY)) $(HOME)/bin/$(BINARY)


.PHONY: install
install: uninstall  ## install
	@VERSION=$(shell cat VERSION) && \
		echo "-M- Installing $$VERSION" && \
		cp -vf bkmr-lsp/target/release/$(BINARY) ~/bin/$(BINARY)$$VERSION && \
		ln -vsf ~/bin/$(BINARY)$$VERSION ~/bin/$(BINARY)
		~/bin/$(BINARY) completion bash > ~/.bash_completions/bkmr-lsp

.PHONY: uninstall
uninstall:  ## uninstall
	-@test -f ~/bin/$(BINARY) && rm -v ~/bin/$(BINARY)
	rm -vf ~/.bash_completions/bkmr-lsp

.PHONY: bump-major
bump-major:  check-github-token  ## bump-major, tag and push
	bump-my-version bump --commit --tag major
	git push
	git push --tags
	@$(MAKE) create-release

.PHONY: bump-minor
bump-minor:  check-github-token  ## bump-minor, tag and push
	bump-my-version bump --commit --tag minor
	git push
	git push --tags
	@$(MAKE) create-release

.PHONY: bump-patch
bump-patch:  check-github-token  ## bump-patch, tag and push
	bump-my-version bump --commit --tag patch
	git push
	git push --tags
	@$(MAKE) create-release

.PHONY: create-release
create-release: check-github-token  ## create a release on GitHub via the gh cli
	@if ! command -v gh &>/dev/null; then \
		echo "You do not have the GitHub CLI (gh) installed. Please create the release manually."; \
		exit 1; \
	else \
		echo "Creating GitHub release for v$(VERSION)"; \
		gh release create "v$(VERSION)" --generate-notes --latest; \
	fi

.PHONY: check-github-token
check-github-token:  ## Check if GITHUB_TOKEN is set
	@if [ -z "$$GITHUB_TOKEN" ]; then \
		echo "GITHUB_TOKEN is not set. Please export your GitHub token before running this command."; \
		exit 1; \
	fi
	@echo "GITHUB_TOKEN is set"
	#@$(MAKE) fix-version  # not working: rustrover deleay


.PHONY: fix-version
fix-version:  ## fix-version of Cargo.toml, re-connect with HEAD
	git add bkmr-lsp/Cargo.lock
	git commit --amend --no-edit
	git tag -f "v$(VERSION)"
	git push --force-with-lease
	git push --tags --force

.PHONY: format
format:  ## format
	bkmr-lsp_DB_URL=../db/bkmr-lsp.db pushd $(pkg_src) && cargo fmt

.PHONY: lint
lint:  ## lint and fix
	pushd $(pkg_src) && cargo clippy --fix  -- -A unused_imports  # avoid errors
	pushd $(pkg_src) && cargo fix --lib -p bkmr-lsp --tests

.PHONY: doc
doc:  ## doc
	@rustup doc --std
	pushd $(pkg_src) && cargo doc --open

################################################################################
# Clean \
CLEAN:  ## ############################################################

.PHONY: clean
clean:clean-rs  ## clean all
	:

.PHONY: clean-build
clean-build: ## remove build artifacts
	rm -fr build/
	rm -fr dist/
	rm -fr .eggs/
	find . \( -path ./env -o -path ./venv -o -path ./.env -o -path ./.venv \) -prune -o -name '*.egg-info' -exec rm -fr {} +
	find . \( -path ./env -o -path ./venv -o -path ./.env -o -path ./.venv \) -prune -o -name '*.egg' -exec rm -f {} +

.PHONY: clean-pyc
clean-pyc: ## remove Python file artifacts
	find . -name '*.pyc' -exec rm -f {} +
	find . -name '*.pyo' -exec rm -f {} +
	find . -name '*~' -exec rm -f {} +
	find . -name '__pycache__' -exec rm -fr {} +

.PHONY: clean-rs
clean-rs:  ## clean-rs
	pushd $(pkg_src) && cargo clean -v

################################################################################
# Misc \
MISC:  ## ############################################################

define PRINT_HELP_PYSCRIPT
import re, sys

for line in sys.stdin:
	match = re.match(r'^([%a-zA-Z0-9_-]+):.*?## (.*)$$', line)
	if match:
		target, help = match.groups()
		if target != "dummy":
			print("\033[36m%-20s\033[0m %s" % (target, help))
endef
export PRINT_HELP_PYSCRIPT

.PHONY: help
help:
	@python -c "$$PRINT_HELP_PYSCRIPT" < $(MAKEFILE_LIST)

debug:  ## debug
	@echo "-D- CODE_DIR: $(CODE_DIR)"


.PHONY: list
list: *  ## list
	@echo $^

.PHONY: list2
%: %.md  ## list2
	@echo $^


%-plan:  ## call with: make <whatever>-plan
	@echo $@ : $*
	@echo $@ : $^
