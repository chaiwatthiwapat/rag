use anyhow::{Context, Result, bail};
use fastembed::{EmbeddingModel, TextEmbedding, TextInitOptions};
use reqwest::{Client, StatusCode};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    env, fs,
    path::{Path, PathBuf},
    time::Duration,
};
use uuid::Uuid;
use walkdir::WalkDir;

const COLLECTION: &str = "documents_e5_small_v1";
const VECTOR_SIZE: usize = 384;
const CHUNK_CHARS: usize = 500;
const CHUNK_OVERLAP: usize = 80;

struct Config {
    qdrant_url: String,
    ollama_url: String,
    ollama_model: String,
}

impl Config {
    fn from_env() -> Self {
        Self {
            qdrant_url: env::var("QDRANT_URL").unwrap_or_else(|_| "http://localhost:6333".into()),
            ollama_url: env::var("OLLAMA_URL").unwrap_or_else(|_| "http://localhost:11434".into()),
            ollama_model: env::var("OLLAMA_MODEL").unwrap_or_else(|_| "qwen2.5:1.5b".into()),
        }
    }
}

#[derive(Deserialize)]
struct QdrantResponse<T> {
    result: T,
}

#[derive(Deserialize)]
struct QueryResult {
    points: Vec<Match>,
}

#[derive(Deserialize)]
struct Match {
    score: f32,
    payload: Value,
}

#[derive(Deserialize)]
struct GenerateResult {
    response: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        bail!("usage: rag ingest <file-or-directory> | rag ask <question>");
    }

    let config = Config::from_env();
    let client = Client::builder()
        .timeout(Duration::from_secs(120))
        .build()?;
    wait_for_qdrant(&client, &config).await?;
    ensure_collection(&client, &config).await?;

    match args[1].as_str() {
        "ingest" => ingest(&client, &config, Path::new(&args[2])).await,
        "ask" => ask(&client, &config, &args[2..].join(" ")).await,
        _ => bail!("unknown command: {} (use ingest or ask)", args[1]),
    }
}

fn embedding_model() -> Result<TextEmbedding> {
    TextEmbedding::try_new(TextInitOptions::new(EmbeddingModel::MultilingualE5Small))
        .context("cannot load FastEmbed model (first run downloads it from Hugging Face)")
}

async fn wait_for_qdrant(client: &Client, config: &Config) -> Result<()> {
    for _ in 0..30 {
        if let Ok(response) = client
            .get(format!("{}/healthz", config.qdrant_url))
            .send()
            .await
        {
            if response.status().is_success() {
                return Ok(());
            }
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    bail!("Qdrant is not ready at {}", config.qdrant_url)
}

async fn ensure_collection(client: &Client, config: &Config) -> Result<()> {
    let url = format!("{}/collections/{COLLECTION}", config.qdrant_url);
    let response = client
        .get(format!("{url}/exists"))
        .send()
        .await?
        .error_for_status()?;
    let body: Value = response.json().await?;
    if body["result"]["exists"] == true {
        return Ok(());
    }
    client
        .put(url)
        .json(&json!({"vectors": {"size": VECTOR_SIZE, "distance": "Cosine"}}))
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}

async fn ingest(client: &Client, config: &Config, path: &Path) -> Result<()> {
    let files = collect_files(path)?;
    if files.is_empty() {
        bail!("no .txt or .md files found at {}", path.display());
    }
    let mut model = embedding_model()?;
    for file in files {
        let source = file.to_string_lossy().to_string();
        let content = fs::read_to_string(&file)
            .with_context(|| format!("cannot read UTF-8 file {}", file.display()))?;
        let chunks = split_text(&content);
        if chunks.is_empty() {
            delete_source(client, config, &source).await?;
            println!("removed empty file: {}", file.display());
            continue;
        }
        let passages: Vec<String> = chunks
            .iter()
            .map(|chunk| format!("passage: {chunk}"))
            .collect();
        let vectors = model.embed(passages, None)?;
        let points: Vec<Value> = vectors.into_iter().zip(chunks.iter()).enumerate().map(|(i, (vector, text))| {
            json!({
                "id": Uuid::new_v5(&Uuid::NAMESPACE_URL, format!("{source}#{i}").as_bytes()).to_string(),
                "vector": vector,
                "payload": {"source": source, "chunk": i, "text": text}
            })
        }).collect();

        let base = format!("{}/collections/{COLLECTION}/points", config.qdrant_url);
        delete_source(client, config, &source).await?;
        for batch in points.chunks(64) {
            client
                .put(format!("{base}?wait=true"))
                .json(&json!({"points": batch}))
                .send()
                .await?
                .error_for_status()?;
        }
        println!("indexed {} ({} chunks)", file.display(), chunks.len());
    }
    Ok(())
}

async fn delete_source(client: &Client, config: &Config, source: &str) -> Result<()> {
    client
        .post(format!(
            "{}/collections/{COLLECTION}/points/delete?wait=true",
            config.qdrant_url
        ))
        .json(&json!({"filter": {"must": [{"key": "source", "match": {"value": source}}]}}))
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}

fn collect_files(path: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in WalkDir::new(path).follow_links(false) {
        let entry = entry?;
        if entry.file_type().is_file()
            && entry.file_name().to_str() != Some("rag-flow.md")
            && entry
                .path()
                .extension()
                .is_some_and(|ext| ext == "txt" || ext == "md")
        {
            files.push(entry.path().canonicalize()?);
        }
    }
    files.sort();
    Ok(files)
}

fn split_text(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut result = Vec::new();
    let mut start = 0;
    while start < chars.len() {
        let end = (start + CHUNK_CHARS).min(chars.len());
        let chunk: String = chars[start..end].iter().collect();
        if !chunk.trim().is_empty() {
            result.push(chunk);
        }
        if end == chars.len() {
            break;
        }
        start = end - CHUNK_OVERLAP;
    }
    result
}

async fn ask(client: &Client, config: &Config, question: &str) -> Result<()> {
    if question.trim().is_empty() {
        bail!("question cannot be empty");
    }
    let mut model = embedding_model()?;
    let vector = model
        .embed(vec![format!("query: {question}")], None)?
        .into_iter()
        .next()
        .context("embedding model returned no vector")?;
    let url = format!(
        "{}/collections/{COLLECTION}/points/query",
        config.qdrant_url
    );
    let response = client
        .post(url)
        .json(&json!({"query": vector, "limit": 4, "with_payload": true}))
        .send()
        .await?
        .error_for_status()?;
    let matches: QdrantResponse<QueryResult> = response.json().await?;
    if matches.result.points.is_empty() {
        println!("No indexed documents yet. Run `ingest` first.");
        return Ok(());
    }

    let context = matches
        .result
        .points
        .iter()
        .enumerate()
        .map(|(i, point)| {
            format!(
                "[{}] {}\n{}",
                i + 1,
                point.payload["source"].as_str().unwrap_or("unknown"),
                point.payload["text"].as_str().unwrap_or("")
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");
    let prompt = format!(
        "Answer in the same language as the question. Use only the numbered context. Answer every part of the question explicitly. Copy names, times, and prices exactly as they appear in the context. Cite the source number after each answer. If a requested fact is missing, say you don't know that fact.\n\nContext:\n{context}\n\nQuestion: {question}\nAnswer:"
    );
    let response = client
        .post(format!("{}/api/generate", config.ollama_url))
        .json(&json!({"model": config.ollama_model, "prompt": prompt, "stream": false, "options": {"temperature": 0}}))
        .send()
        .await?;
    if response.status() == StatusCode::NOT_FOUND {
        bail!(
            "Ollama model '{}' is missing; run `docker compose exec ollama ollama pull {}`",
            config.ollama_model,
            config.ollama_model
        );
    }
    let response = response
        .error_for_status()?
        .json::<GenerateResult>()
        .await?;
    println!("{}\n\nSources:", response.response.trim());
    for (i, point) in matches.result.points.iter().enumerate() {
        println!(
            "[{}] {} (chunk {}, score {:.3})",
            i + 1,
            point.payload["source"].as_str().unwrap_or("unknown"),
            point.payload["chunk"].as_u64().unwrap_or(0),
            point.score
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunks_preserve_unicode_and_overlap() {
        let text = "ก".repeat(600);
        let chunks = split_text(&text);
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].chars().count(), 500);
        assert_eq!(chunks[1].chars().count(), 180);
        assert_eq!(
            &chunks[0][chunks[0].len() - 80 * "ก".len()..],
            &chunks[1][..80 * "ก".len()]
        );
    }
}
