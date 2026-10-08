.DEFAULT_GOAL := help
.PHONY: help setup up pull-model build ingest chat test down

OLLAMA_MODEL ?= qwen2.5:1.5b
DOCS ?= /docs
q ?=
export OLLAMA_MODEL DOCS q

help:
	@printf '%s\n' \
		'make setup                         Start services, pull model, build, and ingest docs' \
		'make ingest [DOCS=/docs]           Index .txt and .md files' \
		'make chat q="..."                 Ask a question (empty q prompts for input)' \
		'make up | pull-model | build       Run individual setup steps' \
		'make test | down                   Run Rust tests or stop services'

setup:
	$(MAKE) up
	$(MAKE) pull-model
	$(MAKE) ingest

up:
	docker compose up -d qdrant ollama

pull-model:
	docker compose up -d ollama
	docker compose exec -T ollama ollama pull "$$OLLAMA_MODEL"

build:
	docker compose build app

ingest:
	docker compose run --rm --build app ingest "$$DOCS"

chat:
	@if [ -z "$$q" ]; then \
		printf 'Question: '; \
		IFS= read -r q || exit 1; \
	fi; \
	if [ -z "$$q" ]; then \
		printf '%s\n' 'Usage: make chat q="your question"' >&2; \
		exit 2; \
	fi; \
	docker compose run --rm --build app ask "$$q"

test:
	cargo test

down:
	docker compose down
