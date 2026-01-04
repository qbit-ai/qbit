//! Message types for multi-modal prompts.
//!
//! This module provides types for user prompts that can include
//! text and image attachments, following patterns from the Vercel AI SDK.

use serde::{Deserialize, Serialize};

/// A part of a multi-modal prompt payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PromptPart {
    /// Text content.
    Text { text: String },
    /// Image attachment (base64 encoded).
    Image {
        /// Base64 encoded data or data URL.
        data: String,
        /// MIME type (e.g., "image/png", "image/jpeg").
        #[serde(default, skip_serializing_if = "Option::is_none")]
        media_type: Option<String>,
        /// Original filename (for display purposes).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        filename: Option<String>,
    },
}

/// A multi-modal prompt payload containing text and/or image parts.
///
/// This is the format expected by the `send_ai_prompt_with_attachments` command.
/// For text-only prompts, the existing `send_ai_prompt_session` can still be used.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptPayload {
    /// The parts of the prompt (text and images).
    pub parts: Vec<PromptPart>,
}

impl PromptPayload {
    /// Create a new payload from a single text prompt.
    ///
    /// This is a convenience method for backward compatibility.
    pub fn from_text(text: impl Into<String>) -> Self {
        Self {
            parts: vec![PromptPart::Text { text: text.into() }],
        }
    }

    /// Extract only the text content from the payload.
    ///
    /// This is used when sending to providers that don't support images.
    /// Multiple text parts are joined with newlines.
    pub fn text_only(&self) -> String {
        self.parts
            .iter()
            .filter_map(|p| match p {
                PromptPart::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Check if the payload contains any image parts.
    pub fn has_images(&self) -> bool {
        self.parts
            .iter()
            .any(|p| matches!(p, PromptPart::Image { .. }))
    }

    /// Get the number of image parts in the payload.
    pub fn image_count(&self) -> usize {
        self.parts
            .iter()
            .filter(|p| matches!(p, PromptPart::Image { .. }))
            .count()
    }

    /// Validate the payload against provider capabilities.
    ///
    /// Returns an error message if validation fails.
    pub fn validate(
        &self,
        max_image_size_bytes: usize,
        supported_formats: &[String],
    ) -> Result<(), String> {
        for part in &self.parts {
            if let PromptPart::Image {
                data, media_type, ..
            } = part
            {
                // Check MIME type
                let mime = media_type.as_deref().unwrap_or("image/png");
                if !supported_formats.iter().any(|f| f == mime) {
                    return Err(format!("Unsupported image type: {}", mime));
                }

                // Estimate size from base64 (actual bytes ≈ len * 3/4)
                // Strip data URL prefix for accurate size estimation
                let base64_part = if data.starts_with("data:") {
                    data.split(',').nth(1).unwrap_or(data)
                } else {
                    data
                };
                let estimated_bytes = base64_part.len() * 3 / 4;

                if estimated_bytes > max_image_size_bytes {
                    return Err(format!(
                        "Image too large: {}MB (max {}MB)",
                        estimated_bytes / 1024 / 1024,
                        max_image_size_bytes / 1024 / 1024
                    ));
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_text() {
        let payload = PromptPayload::from_text("Hello world");
        assert_eq!(payload.text_only(), "Hello world");
        assert!(!payload.has_images());
        assert_eq!(payload.image_count(), 0);
    }

    #[test]
    fn test_has_images() {
        let payload = PromptPayload {
            parts: vec![
                PromptPart::Text {
                    text: "Look at this:".to_string(),
                },
                PromptPart::Image {
                    data: "aGVsbG8=".to_string(), // "hello" in base64
                    media_type: Some("image/png".to_string()),
                    filename: Some("test.png".to_string()),
                },
            ],
        };
        assert!(payload.has_images());
        assert_eq!(payload.image_count(), 1);
        assert_eq!(payload.text_only(), "Look at this:");
    }

    #[test]
    fn test_validate_unsupported_format() {
        let payload = PromptPayload {
            parts: vec![PromptPart::Image {
                data: "aGVsbG8=".to_string(),
                media_type: Some("image/bmp".to_string()),
                filename: None,
            }],
        };
        let supported = vec!["image/png".to_string(), "image/jpeg".to_string()];
        let result = payload.validate(10 * 1024 * 1024, &supported);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unsupported image type"));
    }

    #[test]
    fn test_serde_roundtrip() {
        let payload = PromptPayload {
            parts: vec![
                PromptPart::Text {
                    text: "Hello".to_string(),
                },
                PromptPart::Image {
                    data: "YWJj".to_string(),
                    media_type: Some("image/png".to_string()),
                    filename: None,
                },
            ],
        };
        let json = serde_json::to_string(&payload).unwrap();
        let parsed: PromptPayload = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.parts.len(), 2);
        assert!(parsed.has_images());
    }
}
