/**
 * Frontend API for the model registry.
 *
 * Provides typed wrappers for fetching model definitions and capabilities
 * from the backend model registry.
 */

import { invoke } from "@tauri-apps/api/core";
import type { AiProvider, ModelCapabilities, OwnedModelDefinition } from "./generated";

// Re-export generated types for convenience
export type { AiProvider, ModelCapabilities, OwnedModelDefinition };

/**
 * Provider metadata for UI display.
 * Note: This is manually defined because the Rust struct uses &'static str
 * which ts-rs doesn't export directly.
 */
export interface ProviderInfo {
  provider: AiProvider;
  name: string;
  icon: string;
  description: string;
}

/**
 * Get all available models, optionally filtered by provider.
 *
 * @param provider - Optional provider to filter by
 * @returns Array of model definitions
 */
export async function getAvailableModels(provider?: AiProvider): Promise<OwnedModelDefinition[]> {
  return invoke("get_available_models", { provider: provider ?? null });
}

/**
 * Get a specific model by its ID.
 *
 * @param modelId - The model ID to look up
 * @returns The model definition, or null if not found
 */
export async function getModelById(modelId: string): Promise<OwnedModelDefinition | null> {
  return invoke("get_model_by_id", { modelId });
}

/**
 * Get capabilities for a specific model.
 *
 * This returns capabilities even for unknown models by using
 * provider-specific defaults.
 *
 * @param provider - The AI provider
 * @param modelId - The model ID
 * @returns The model's capabilities
 */
export async function getModelCapabilities(
  provider: AiProvider,
  modelId: string
): Promise<ModelCapabilities> {
  return invoke("get_model_capabilities_command", { provider, modelId });
}

/**
 * Get information about all available providers.
 *
 * @returns Array of provider information for UI display
 */
export async function getProviders(): Promise<ProviderInfo[]> {
  return invoke("get_providers");
}

/**
 * Get models grouped by provider.
 *
 * @returns Map of provider to their models
 */
export async function getModelsGroupedByProvider(): Promise<
  Map<AiProvider, OwnedModelDefinition[]>
> {
  const models = await getAvailableModels();
  const grouped = new Map<AiProvider, OwnedModelDefinition[]>();

  for (const model of models) {
    const existing = grouped.get(model.provider) ?? [];
    existing.push(model);
    grouped.set(model.provider, existing);
  }

  return grouped;
}

/**
 * Check if a model supports a specific capability.
 *
 * @param capabilities - The model's capabilities
 * @param capability - The capability to check
 * @returns Whether the capability is supported
 */
export function hasCapability(
  capabilities: ModelCapabilities,
  capability: keyof ModelCapabilities
): boolean {
  const value = capabilities[capability];
  if (typeof value === "boolean") {
    return value;
  }
  if (typeof value === "number") {
    return value > 0;
  }
  return false;
}

/**
 * Get provider display name from provider ID.
 *
 * @param providers - Array of provider info (from getProviders())
 * @param provider - The provider ID
 * @returns The display name, or the provider ID if not found
 */
export function getProviderDisplayName(providers: ProviderInfo[], provider: AiProvider): string {
  return providers.find((p) => p.provider === provider)?.name ?? provider;
}

/**
 * Get provider icon from provider ID.
 *
 * @param providers - Array of provider info (from getProviders())
 * @param provider - The provider ID
 * @returns The icon, or empty string if not found
 */
export function getProviderIcon(providers: ProviderInfo[], provider: AiProvider): string {
  return providers.find((p) => p.provider === provider)?.icon ?? "";
}
