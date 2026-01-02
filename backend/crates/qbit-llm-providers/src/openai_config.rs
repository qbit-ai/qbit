//! OpenAI-specific configuration types.
//!
//! This module contains configuration types for OpenAI-specific features
//! like the web_search_preview tool.

use serde::{Deserialize, Serialize};

/// Configuration for OpenAI's web_search_preview tool.
///
/// This is a server-side tool that OpenAI executes during inference,
/// similar to Anthropic's native web tools.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiWebSearchConfig {
    /// Search context size: "low", "medium", or "high".
    ///
    /// - "low": Faster and cheaper, but may be less accurate
    /// - "medium": Balanced (default)
    /// - "high": Better results, but slower and more expensive
    #[serde(default = "default_search_context_size")]
    pub search_context_size: String,

    /// User location for localized search results.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_location: Option<OpenAiUserLocation>,
}

fn default_search_context_size() -> String {
    "medium".to_string()
}

impl Default for OpenAiWebSearchConfig {
    fn default() -> Self {
        Self {
            search_context_size: default_search_context_size(),
            user_location: None,
        }
    }
}

impl OpenAiWebSearchConfig {
    /// Create a new config with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a config with a specific search context size.
    pub fn with_context_size(size: &str) -> Self {
        Self {
            search_context_size: size.to_string(),
            user_location: None,
        }
    }

    /// Set the user location for localized results.
    pub fn with_location(mut self, location: OpenAiUserLocation) -> Self {
        self.user_location = Some(location);
        self
    }

    /// Convert to JSON for the OpenAI API request.
    pub fn to_tool_json(&self) -> serde_json::Value {
        let mut tool = serde_json::json!({
            "type": "web_search_preview",
            "search_context_size": self.search_context_size,
        });

        if let Some(ref loc) = self.user_location {
            tool["user_location"] = loc.to_json();
        }

        tool
    }
}

/// User location for OpenAI web search localization.
///
/// Used to refine search results based on geography.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiUserLocation {
    /// Location type - always "approximate" for OpenAI.
    #[serde(rename = "type", default = "default_location_type")]
    pub location_type: String,

    /// Two-letter ISO country code (e.g., "US", "GB").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,

    /// City name (e.g., "New York", "London").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,

    /// Region or state (e.g., "New York", "California").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
}

fn default_location_type() -> String {
    "approximate".to_string()
}

impl OpenAiUserLocation {
    /// Create an approximate location with just a country code.
    pub fn country(code: &str) -> Self {
        Self {
            location_type: "approximate".to_string(),
            country: Some(code.to_string()),
            city: None,
            region: None,
        }
    }

    /// Create a full approximate location.
    pub fn approximate(country: &str, city: Option<&str>, region: Option<&str>) -> Self {
        Self {
            location_type: "approximate".to_string(),
            country: Some(country.to_string()),
            city: city.map(|s| s.to_string()),
            region: region.map(|s| s.to_string()),
        }
    }

    /// Convert to JSON for the API request.
    pub fn to_json(&self) -> serde_json::Value {
        let mut loc = serde_json::json!({
            "type": self.location_type,
        });

        if let Some(ref c) = self.country {
            loc["country"] = serde_json::json!(c);
        }
        if let Some(ref c) = self.city {
            loc["city"] = serde_json::json!(c);
        }
        if let Some(ref r) = self.region {
            loc["region"] = serde_json::json!(r);
        }

        loc
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = OpenAiWebSearchConfig::default();
        assert_eq!(config.search_context_size, "medium");
        assert!(config.user_location.is_none());
    }

    #[test]
    fn test_config_with_context_size() {
        let config = OpenAiWebSearchConfig::with_context_size("high");
        assert_eq!(config.search_context_size, "high");
    }

    #[test]
    fn test_config_to_tool_json() {
        let config = OpenAiWebSearchConfig::with_context_size("low");
        let json = config.to_tool_json();

        assert_eq!(json["type"], "web_search_preview");
        assert_eq!(json["search_context_size"], "low");
    }

    #[test]
    fn test_config_with_location() {
        let location = OpenAiUserLocation::approximate("US", Some("New York"), Some("NY"));
        let config = OpenAiWebSearchConfig::with_context_size("medium").with_location(location);

        let json = config.to_tool_json();
        assert_eq!(json["user_location"]["type"], "approximate");
        assert_eq!(json["user_location"]["country"], "US");
        assert_eq!(json["user_location"]["city"], "New York");
        assert_eq!(json["user_location"]["region"], "NY");
    }
}
