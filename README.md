# Rust RAG with FastEmbed

```text
-- text

This project is a local RAG command-line example. Rust uses FastEmbed to
create embeddings, Qdrant to store and retrieve document chunks, and Ollama
to generate answers from the retrieved text.
```

## Documentation

| Document | Purpose |
|---|---|
| [System flow](docs/rag-flow.md) | Ingestion and question-answering paths. |

## Components

| Component | Responsibility | Default |
|---|---|---|
| Rust CLI | Ingest documents and answer questions | `app` service |
| FastEmbed | Embed documents and questions | `multilingual-e5-small` |
| Qdrant | Store vectors and retrieve similar chunks | `documents_e5_small_v1` collection |
| Ollama | Generate answers from retrieved context | `qwen2.5:1.5b` |

## Quick start

```text
-- text 

Docker and Docker Compose are required. The first run downloads the container
images, the embedding model, and the Ollama model.

make setup
make chat q="When does the shop open, and how much is an iced latte?"

make setup starts Qdrant and Ollama, pulls the default Ollama model, builds
the Rust app, and ingests the files in docs/.
```

## Commands

| Command | Result |
|---|---|
| `make ingest` | Index all supported files in `docs/` and its subdirectories. |
| `make ingest DOCS=/docs/example.md` | Index one file from the container's `/docs` mount. |
| `make chat q="..."` | Ask a question about indexed documents. |
| `make chat q=""` | Prompt for a question interactively. |
| `make test` | Run the Rust unit tests on the host. |
| `make down` | Stop the Compose services. |

```bash
-- bash

# To change the answer model, pull it before asking a question:

OLLAMA_MODEL=qwen2.5:3b make pull-model
OLLAMA_MODEL=qwen2.5:3b make chat q="When does the shop open?"
```

## Document contract

```text
-- text

The ingester reads UTF-8 .md and .txt files recursively. It splits text into
500-character chunks with an 80-character overlap. Re-ingesting a file
replaces that file's previous chunks. The flow diagram at docs/rag-flow.md
is excluded from ingestion.

Answers show the retrieved source path, chunk number, and similarity score.
Qdrant, Ollama, and FastEmbed data persist in Docker volumes after make down.
Running docker compose down -v removes those volumes.
```

## Local development

```bash
-- bash

cargo test
QDRANT_URL=http://localhost:6333 OLLAMA_URL=http://localhost:11435 cargo run -- ingest docs

# This example handles short text documents. It does not parse PDFs, enforce
# document access controls, or remove indexed chunks when a source file is
# deleted from docs/.
```
