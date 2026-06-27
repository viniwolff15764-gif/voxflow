use reqwest::multipart;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct WhisperResponse {
    pub text: String,
}

#[derive(Debug, Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f64,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessageResponse,
}

#[derive(Debug, Deserialize)]
struct ChatMessageResponse {
    content: String,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

pub async fn transcribe(
    api_key: &str,
    audio_data: Vec<u8>,
    language: &str,
    model: &str,
) -> Result<String, String> {
    let client = reqwest::Client::new();

    let audio_part = multipart::Part::bytes(audio_data)
        .file_name("audio.wav")
        .mime_str("audio/wav")
        .map_err(|e| e.to_string())?;

    let form = multipart::Form::new()
        .text("model", model.to_string())
        .text("language", language.to_string())
        .text("response_format", "json")
        .text("temperature", "0")
        .part("file", audio_part);

    let response = client
        .post("https://api.groq.com/openai/v1/audio/transcriptions")
        .header("Authorization", format!("Bearer {}", api_key))
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Groq API error {}: {}", status, body));
    }

    let result: WhisperResponse = response
        .json()
        .await
        .map_err(|e| format!("Parse error: {}", e))?;

    Ok(result.text)
}

pub async fn chat_command(api_key: &str, model: &str, instruction: &str, text: &str) -> Result<String, String> {
    let client = reqwest::Client::new();

    let request = ChatRequest {
        model: model.to_string(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: "Você é um assistente que modifica texto conforme instruções. Responda APENAS com o texto modificado, sem explicações.".to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: format!("Instrução: {}\n\nTexto:\n{}", instruction, text),
            },
        ],
        temperature: 0.3,
    };

    let response = client
        .post("https://api.groq.com/openai/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Groq API error {}: {}", status, body));
    }

    let result: ChatResponse = response
        .json()
        .await
        .map_err(|e| format!("Parse error: {}", e))?;

    result
        .choices
        .first()
        .map(|c| c.message.content.clone())
        .ok_or_else(|| "No response from LLM".to_string())
}
