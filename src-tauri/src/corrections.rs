use serde::{Deserialize, Serialize};

/// Represents a single correction made by the AI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Correction {
    pub original: String,
    pub corrected: String,
    pub category: CorrectionCategory,
    pub severity: CorrectionSeverity,
    pub explanation: String,
}

/// Category of a correction
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CorrectionCategory {
    Grammar,
    Spelling,
    Punctuation,
    Tone,
    Style,
    Enhancement,
}

/// Severity level of a correction
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CorrectionSeverity {
    Error,
    Warning,
    Suggestion,
}

/// The response returned from the Gemini API after processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrectionResponse {
    pub corrected_text: String,
    pub has_changes: bool,
    pub corrections: Vec<Correction>,
}

/// Result sent to the frontend for display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrectionResult {
    pub original_text: String,
    pub corrected_text: String,
    pub has_changes: bool,
    pub corrections: Vec<Correction>,
    pub num_corrections: usize,
}

impl CorrectionResult {
    pub fn from_response(original_text: String, response: CorrectionResponse) -> Self {
        let num_corrections = response.corrections.len();
        CorrectionResult {
            original_text,
            corrected_text: response.corrected_text,
            has_changes: response.has_changes,
            corrections: response.corrections,
            num_corrections,
        }
    }

    pub fn no_changes(original_text: String) -> Self {
        CorrectionResult {
            original_text: original_text.clone(),
            corrected_text: original_text,
            has_changes: false,
            corrections: Vec::new(),
            num_corrections: 0,
        }
    }
}
