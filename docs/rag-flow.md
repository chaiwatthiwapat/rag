# RAG System Flow

```text
-- text

This document describes the two runtime paths of the Rust RAG example.
The diagrams use fenced text blocks so their layout remains readable in
Markdown previews.
```

## Components

| Component | Role in this flow |
|---|---|
| Rust CLI | Reads files, builds prompts, and prints answers with sources. |
| FastEmbed | Creates document and question vectors with `multilingual-e5-small`. |
| Qdrant | Stores vectors and retrieves the nearest chunks. |
| Ollama | Generates an answer from the retrieved context. |

## 1. Ingest documents (`make ingest`)

```text
                 +---------------------------+
                 | START: make ingest        |
                 +-------------+-------------+
                               |
                               v
                 +---------------------------+
                 | Find .md/.txt in docs/    |
                 +-------------+-------------+
                               |
                               v
                         < Any files? >
                        /              \
                     no/                \yes
                      v                  v
           +------------------+   +---------------------------+
           | END: no files    |   | Read UTF-8; split 500/80  |
           +------------------+   +-------------+-------------+
                                              |
                                              v
                                +---------------------------+
                                | FastEmbed: passage        |
                                |            -> vector      |
                                +-------------+-------------+
                                              |
                                              v
                                +---------------------------+
                                | Delete old chunks         |
                                | Store in Qdrant           |
                                +-------------+-------------+
                                              |
                                              v
                                        < More files? >
                                       /              \
                                    yes/                \no
                                      v                  v
                            (loop: read next)   +------------------+
                                                | END: indexed     |
                                                +------------------+
```

```text
-- text

Each Qdrant point stores a vector and the payload fields source, chunk,
and text. FastEmbed receives document text with a "passage: " prefix.
Re-ingesting a file deletes its previous chunks before storing the new ones.
```

## 2. Answer questions (`make chat q="question"`)

```text
                 +---------------------------+
                 | START: make chat q="..."  |
                 +-------------+-------------+
                               |
                               v
                 +---------------------------+
                 | FastEmbed: query -> vec   |
                 +-------------+-------------+
                               |
                               v
                 +---------------------------+
                 | Qdrant: top 4 chunks      |
                 +-------------+-------------+
                               |
                               v
                         < Hits found? >
                        /              \
                     no/                \yes
                      v                  v
           +------------------+   +---------------------------+
           | END: ingest docs |   | Rust: build prompt        |
           | first            |   | from question + chunks    |
           +------------------+   +-------------+-------------+
                                              |
                                              v
                                +---------------------------+
                                | Ollama: generate answer   |
                                +-------------+-------------+
                                              |
                                              v
                                      < Model found? >
                                     /               \
                                  no/                 \yes
                                    v                   v
                          +------------------+   +-------------------+
                          | END: pull model  |   | END: answer +     |
                          | then retry       |   | sources + scores  |
                          +------------------+   +-------------------+
```

```text
-- text

FastEmbed receives questions with a "query: " prefix. Ollama uses
qwen2.5:1.5b by default. Retrieval and answer generation are separate
steps: Qdrant may find the right source even when the model phrases its
answer poorly.
```

## Response and failure behavior

| Condition | Behavior |
|---|---|
| Qdrant returns no chunks | The CLI asks the user to ingest documents first; Ollama is not called. |
| Ollama model is missing | The CLI reports the model name and the pull command. |
| An answer is generated | The CLI prints the answer, then each retrieved source path, chunk number, and similarity score. |

```text
-- text

This file lives in docs/, but the ingester excludes rag-flow.md so the
diagram does not become searchable source content.
```
