use crate::corrections::CorrectionResponse;
use crate::settings::Strictness;
use reqwest::Client;
use serde::Serialize;
use serde_json::Value;

const GEMINI_API_URL: &str =
    "https://generativelanguage.googleapis.com/v1beta/models/gemini-flash-lite-latest:generateContent";

/// Build the system prompt based on strictness level
fn build_system_prompt(strictness: &Strictness) -> String {
    let strictness_instruction = match strictness {
        Strictness::Relaxed => {
            "Correction level: RELAXED. Only fix clear grammar errors and spelling mistakes. \
             Leave tone and style completely alone. Do not suggest rewording. \
             This text is likely informal (chat, quick note). Preserve casual language like \
             'gonna', 'wanna', etc."
        }
        Strictness::Balanced => {
            "Correction level: BALANCED. Fix grammar, spelling, and article/preposition errors. \
             Offer light tone suggestions only if the text sounds clearly unnatural. \
             Preserve the author's original voice."
        }
        Strictness::Strict => {
            "Correction level: STRICT. Provide full corrections including grammar, spelling, \
             punctuation, tone, wordiness, formality, and naturalness. \
             Suggest improvements for awkward phrasing. Suitable for emails and professional writing."
        }
    };

    format!(
        r#"You are GrammarLens, an English grammar correction assistant designed to help non-native English speakers improve their writing. Your corrections should be:

1. Accurate — only fix actual errors, don't change correct text.
2. Minimal — preserve the user's voice and intent. Don't rewrite sentences unnecessarily.
3. Educational — explanations should be clear, concise, and written for someone actively learning English (B1-C1 level).
4. Context-aware — consider informal contexts (chat messages, texts) vs formal (emails, documents). Don't over-correct casual writing.

{}

Special rules:
- Recognize and skip code snippets, variable names, URLs, file paths — do not correct them.
- If the text contains intentional slang or abbreviations in casual context, don't correct them but you may note they are informal.
- If the text mixes languages, only correct the English portions.
- If the text is empty or too short to meaningfully check, return has_changes: false.

You MUST respond with ONLY valid JSON matching this exact schema, with no additional text:
{{
  "corrected_text": "the fully corrected text",
  "has_changes": true or false,
  "corrections": [
    {{
      "original": "the exact wrong part from the original",
      "corrected": "the fixed version",
      "category": "grammar" or "spelling" or "punctuation" or "tone" or "style",
      "severity": "error" or "warning" or "suggestion",
      "explanation": "Clear, simple explanation of why this is wrong and the grammar rule. Written for someone learning English."
    }}
  ]
}}"#,
        strictness_instruction
    )
}

/// Request body for the Gemini API
#[derive(Serialize)]
struct GeminiRequest {
    contents: Vec<Content>,
    system_instruction: SystemInstruction,
    generation_config: GenerationConfig,
}

#[derive(Serialize)]
struct SystemInstruction {
    parts: Vec<Part>,
}

#[derive(Serialize)]
struct Content {
    parts: Vec<Part>,
}

#[derive(Serialize)]
struct Part {
    text: String,
}

#[derive(Serialize)]
struct GenerationConfig {
    response_mime_type: String,
    temperature: f32,
}

/// Call the Gemini Flash API with the given text and return corrections
pub async fn check_grammar(
    api_key: &str,
    text: &str,
    strictness: &Strictness,
) -> Result<CorrectionResponse, String> {
    if api_key.is_empty() {
        return Err(
            "API key not configured. Please set your Gemini API key in Settings.".to_string(),
        );
    }

    if text.trim().is_empty() {
        return Ok(CorrectionResponse {
            corrected_text: text.to_string(),
            has_changes: false,
            corrections: Vec::new(),
        });
    }

    let client = Client::new();
    let system_prompt = build_system_prompt(strictness);

    let request_body = GeminiRequest {
        system_instruction: SystemInstruction {
            parts: vec![Part {
                text: system_prompt,
            }],
        },
        contents: vec![Content {
            parts: vec![Part {
                text: text.to_string(),
            }],
        }],
        generation_config: GenerationConfig {
            response_mime_type: "application/json".to_string(),
            temperature: 0.2,
        },
    };

    let url = format!("{}?key={}", GEMINI_API_URL, api_key);

    let response = client
        .post(&url)
        .json(&request_body)
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    let status = response.status();
    if !status.is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("Gemini API error ({}): {}", status, error_text));
    }

    let response_json: Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse API response: {}", e))?;

    // Extract the text content from the Gemini response
    let text_content = response_json["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .ok_or("Unexpected API response structure")?;

    // Parse the JSON content from the model's response
    let correction_response: CorrectionResponse =
        serde_json::from_str(text_content).map_err(|e| {
            format!(
                "Failed to parse correction JSON: {}. Raw: {}",
                e, text_content
            )
        })?;

    Ok(correction_response)
}
