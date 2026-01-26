//! Tauri commands for model registry access.
//!
//! These commands expose the model registry to the frontend, allowing
//! the UI to dynamically fetch available models and their capabilities.

use qbit_models::{
    get_all_models_owned, get_all_provider_info, get_model_capabilities, get_model_owned,
    get_models_for_provider_owned, AiProvider, ModelCapabilities, OwnedModelDefinition,
    ProviderInfo,
};

/// Get all available models from all providers.
#[tauri::command]
pub async fn get_available_models(
    provider: Option<AiProvider>,
) -> Result<Vec<OwnedModelDefinition>, String> {
    match provider {
        Some(p) => Ok(get_models_for_provider_owned(p)),
        None => Ok(get_all_models_owned()),
    }
}

/// Get a specific model by ID.
#[tauri::command]
pub async fn get_model_by_id(model_id: String) -> Result<Option<OwnedModelDefinition>, String> {
    Ok(get_model_owned(&model_id))
}

/// Get capabilities for a specific model.
///
/// This returns capabilities even for unknown models by using
/// provider-specific defaults.
#[tauri::command]
pub async fn get_model_capabilities_command(
    provider: AiProvider,
    model_id: String,
) -> Result<ModelCapabilities, String> {
    Ok(get_model_capabilities(provider, &model_id))
}

/// Get information about all available providers.
#[tauri::command]
pub async fn get_providers() -> Result<Vec<ProviderInfo>, String> {
    Ok(get_all_provider_info())
}
