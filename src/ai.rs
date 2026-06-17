//! Local AI assistant via the Ollama HTTP API (loopback, no auth).
//!
//! A LaTeX-focused chat assistant backed by a local Ollama model (default
//! `gemma4:12b-it-qat`). Streaming matters: a local model's first request loads
//! weights (seconds) and then emits tokens incrementally, so we surface deltas
//! as they arrive rather than blocking on the full response.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::UnboundedSender;

/// One chat turn.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

impl ChatMessage {
    pub fn user(content: impl Into<String>) -> Self {
        Self { role: "user".into(), content: content.into() }
    }
    pub fn assistant(content: impl Into<String>) -> Self {
        Self { role: "assistant".into(), content: content.into() }
    }
    pub fn system(content: impl Into<String>) -> Self {
        Self { role: "system".into(), content: content.into() }
    }
}

/// Events streamed from a chat request up to the app loop.
#[derive(Debug)]
pub enum AiEvent {
    /// A chunk of assistant text.
    Delta(String),
    /// The response finished.
    Done,
    /// Something went wrong (connection, HTTP status, …).
    Error(String),
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: &'a [ChatMessage],
    stream: bool,
}

/// JSON body for an `/api/chat` request.
pub fn chat_body(model: &str, messages: &[ChatMessage], stream: bool) -> String {
    serde_json::to_string(&ChatRequest { model, messages, stream }).unwrap_or_default()
}

#[derive(Deserialize)]
struct ChunkResponse {
    #[serde(default)]
    message: Option<ChunkMessage>,
    #[serde(default)]
    done: bool,
}
#[derive(Deserialize, Default)]
struct ChunkMessage {
    #[serde(default)]
    content: String,
}

/// Parse one NDJSON line from a streaming `/api/chat` response into
/// `(content_delta, done)`.
pub fn parse_chunk(line: &str) -> Option<(String, bool)> {
    let r: ChunkResponse = serde_json::from_str(line).ok()?;
    let content = r.message.unwrap_or_default().content;
    Some((content, r.done))
}

/// Stream a chat completion, sending each delta over `tx`. Stops early if
/// `cancel` is set. Always ends with `Done` or `Error`.
pub async fn chat_stream(
    host: String,
    model: String,
    messages: Vec<ChatMessage>,
    cancel: Arc<AtomicBool>,
    tx: UnboundedSender<AiEvent>,
) {
    let url = format!("{}/api/chat", host.trim_end_matches('/'));
    let body = chat_body(&model, &messages, true);

    let resp = reqwest::Client::new()
        .post(&url)
        .header("Content-Type", "application/json")
        .body(body)
        .send()
        .await;

    let resp = match resp {
        Ok(r) if r.status().is_success() => r,
        Ok(r) => {
            let _ = tx.send(AiEvent::Error(format!("Ollama returned {}", r.status())));
            return;
        }
        Err(e) => {
            let _ = tx.send(AiEvent::Error(format!(
                "can't reach Ollama at {host} — is it running? ({e})"
            )));
            return;
        }
    };

    let mut stream = resp.bytes_stream();
    let mut buf = String::new();
    while let Some(item) = stream.next().await {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        let bytes = match item {
            Ok(b) => b,
            Err(e) => {
                let _ = tx.send(AiEvent::Error(format!("stream error: {e}")));
                return;
            }
        };
        buf.push_str(&String::from_utf8_lossy(&bytes));

        // Ollama streams newline-delimited JSON objects.
        while let Some(pos) = buf.find('\n') {
            let line: String = buf.drain(..=pos).collect();
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Some((delta, done)) = parse_chunk(line) {
                if !delta.is_empty() {
                    let _ = tx.send(AiEvent::Delta(delta));
                }
                if done {
                    let _ = tx.send(AiEvent::Done);
                    return;
                }
            }
        }
    }
    let _ = tx.send(AiEvent::Done);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_chat_body() {
        let msgs = vec![ChatMessage::system("be terse"), ChatMessage::user("hi")];
        let body = chat_body("gemma4:12b-it-qat", &msgs, true);
        assert!(body.contains("\"model\":\"gemma4:12b-it-qat\""));
        assert!(body.contains("\"stream\":true"));
        assert!(body.contains("\"role\":\"system\"") && body.contains("\"role\":\"user\""));
    }

    #[test]
    fn parses_stream_chunk() {
        let (d, done) =
            parse_chunk(r#"{"message":{"role":"assistant","content":"Hi"},"done":false}"#).unwrap();
        assert_eq!(d, "Hi");
        assert!(!done);

        let (_, done) = parse_chunk(r#"{"message":{"content":""},"done":true}"#).unwrap();
        assert!(done);

        assert!(parse_chunk("not json").is_none());
    }
}
