# Provider & Model Configuration Analysis

## Executive Summary

The current provider and model configuration is spread across **6+ files** with significant duplication between frontend and backend. Adding a new provider requires changes in at least **5 locations**. This document outlines the current architecture and proposes a simplification plan.

---

## Current Architecture

### 1. Backend Configuration

#### Provider Enum (`backend/crates/qbit-settings/src/schema.rs`)
```rust
pub enum AiProvider {
    VertexAi,      // default
    Openrouter,
    Anthropic,
    Openai,
    Ollama,
    Gemini,
    Groq,
    Xai,
    ZaiSdk,
}
```

Each provider has a dedicated settings struct (e.g., `VertexAiSettings`, `OpenAiSettings`) containing:
- Credentials (API key, service account path, etc.)
- Provider-specific options (base_url, web_search settings, etc.)
- `show_in_selector: bool` - controls visibility in UI

**Key observation**: The backend does NOT maintain a list of available models - it treats models as opaque strings.

#### LLM Client Enum (`backend/crates/qbit-llm-providers/src/lib.rs`)
```rust
pub enum LlmClient {
    VertexAnthropic(rig_anthropic_vertex::CompletionModel),
    RigOpenRouter(rig_openrouter::CompletionModel),
    RigOpenAi(rig_openai::completion::CompletionModel),
    RigOpenAiResponses(rig_openai::responses_api::ResponsesCompletionModel),
    OpenAiReasoning(rig_openai_responses::CompletionModel),  // Custom for o1/o3/gpt-5
    RigAnthropic(rig_anthropic::completion::CompletionModel),
    RigOllama(rig_ollama::CompletionModel<reqwest::Client>),
    RigGemini(rig_gemini::completion::CompletionModel),
    RigGroq(rig_groq::CompletionModel<reqwest::Client>),
    RigXai(rig_xai::completion::CompletionModel<reqwest::Client>),
    RigZaiSdk(rig_zai_sdk::CompletionModel),
    Mock,
}
```

**Note**: OpenAI has 3 different client variants based on model type!

#### Model Capabilities Detection (`backend/crates/qbit-llm-providers/src/model_capabilities.rs`)

**This is where backend hardcodes model knowledge**:

```rust
// Temperature support detection
pub fn model_supports_temperature(provider: &str, model: &str) -> bool {
    match provider {
        "openai" | "openai_responses" => {
            // Codex, o-series, gpt-5 don't support temperature
            if model.contains("codex") { return false; }
            if model.starts_with("o1") || model.starts_with("o3") || model.starts_with("o4") {
                return false;
            }
            if model.starts_with("gpt-5") { return false; }
            true
        }
        _ => true,
    }
}

// Web search support
const OPENAI_WEB_SEARCH_MODELS: &[&str] = &[
    "gpt-4o", "gpt-4o-mini", "chatgpt-4o-latest",
    "gpt-4.1", "gpt-4.1-mini", "gpt-4.1-nano",
    "gpt-5", "gpt-5.1", "gpt-5.2", "gpt-5-mini", "gpt-5-nano",
];

// Vision support detection
pub fn detect_vision(provider: &str, model: &str) -> VisionCapabilities { ... }

// Thinking/reasoning history detection
fn detect_thinking_history_support(provider: &str, model: &str) -> bool { ... }
```

#### ProviderConfig Enum (`backend/crates/qbit-llm-providers/src/lib.rs`)
```rust
#[derive(Deserialize)]
#[serde(tag = "provider", rename_all = "snake_case")]
pub enum ProviderConfig {
    VertexAi { workspace, model, credentials_path?, project_id, location },
    Openrouter { workspace, model, api_key },
    Openai { workspace, model, api_key, base_url?, reasoning_effort?, enable_web_search, ... },
    Anthropic { workspace, model, api_key },
    Ollama { workspace, model, base_url? },
    Gemini { workspace, model, api_key },
    Groq { workspace, model, api_key },
    Xai { workspace, model, api_key },
    ZaiSdk { workspace, model, api_key, base_url?, source_channel? },
}
```

Used for Tauri command deserialization from frontend.

#### LLM Client Factory (`backend/crates/qbit-ai/src/llm_client.rs`)

Two layers of client creation:

1. **Top-level functions** (`create_openai_components()`, `create_vertex_components()`, etc.)
   - Called during session initialization
   - Create `AgentBridgeComponents` with full context (workspace, tool registry, etc.)
   - Contains provider-specific logic (e.g., OpenAI reasoning model detection)

2. **`LlmClientFactory`** - For sub-agent model overrides
   ```rust
   pub struct LlmClientFactory {
       cache: RwLock<HashMap<(String, String), Arc<LlmClient>>>,
       settings_manager: Arc<SettingsManager>,
       workspace: PathBuf,
   }

   impl LlmClientFactory {
       pub async fn get_or_create(&self, provider: &str, model: &str) -> Result<Arc<LlmClient>>;
   }
   ```
   - Reads credentials from settings
   - Caches clients by (provider, model) key
   - Duplicates much of the logic from top-level functions

#### Reasoning Model Detection (`rig-openai-responses` crate)
```rust
pub fn is_reasoning_model(model: &str) -> bool {
    let model_lower = model.to_lowercase();
    model_lower.starts_with("o1")
        || model_lower.starts_with("o3")
        || model_lower.starts_with("o4")
        || model_lower.starts_with("gpt-5")
}
```

This is checked in multiple places to route to the correct client variant.

---

### 2. Frontend Configuration

#### Model ID Constants (`frontend/lib/ai.ts`)
```typescript
export const VERTEX_AI_MODELS = {
  CLAUDE_OPUS_4_5: "claude-opus-4-5@20251101",
  CLAUDE_SONNET_4_5: "claude-sonnet-4-5@20250929",
  CLAUDE_HAIKU_4_5: "claude-haiku-4-5@20251001",
} as const;

export const OPENAI_MODELS = {
  GPT_5_2: "gpt-5.2",
  GPT_5_1: "gpt-5.1",
  // ... 15+ more models
} as const;

// Similar constants for: ANTHROPIC, OLLAMA, GEMINI, GROQ, XAI, ZAI_SDK
```

#### Model Groups (`frontend/lib/models.ts`)
Two parallel data structures for UI rendering:

1. **`PROVIDER_GROUPS`** - Flat list for simple dropdowns
2. **`PROVIDER_GROUPS_NESTED`** - Hierarchical for sub-menus (OpenAI reasoning variants)

Each group contains:
```typescript
interface ProviderGroup {
  provider: AiProvider;
  providerName: string;
  icon: string;
  models: ModelInfo[];
}
```

#### Provider Metadata (`frontend/components/Settings/ProviderSettings.tsx`)
```typescript
const PROVIDERS: ProviderConfig[] = [
  {
    id: "anthropic",
    name: "Anthropic",
    icon: "🔶",
    description: "Direct Claude API access",
    getConfigured: (s) => !!s.anthropic.api_key,
  },
  // ... 8 more providers
];
```

#### TypeScript Settings Types (`frontend/lib/settings.ts`)
Mirrors the Rust schema exactly - must be kept in sync manually.

---

### 3. UI Components That Display Providers/Models

| Component | Purpose | Data Source |
|-----------|---------|-------------|
| `ProviderSettings.tsx` | Provider config cards | Hardcoded `PROVIDERS` array |
| `ModelSelector.tsx` | Default model dropdown in settings | `PROVIDER_GROUPS_NESTED` |
| `InputStatusRow.tsx` | Footer model selector | `PROVIDER_GROUPS` + `PROVIDER_GROUPS_NESTED` |
| `AiSettings.tsx` | Synthesis backend config | **Hardcoded model lists (separate!)** |

---

## Duplication Map

| Data | Backend Location | Frontend Location(s) |
|------|------------------|---------------------|
| Provider enum/type | `schema.rs:14-28` | `settings.ts:88-97` |
| Provider names/icons | N/A | `ProviderSettings.tsx:41-105`, `models.ts` (in PROVIDER_GROUPS) |
| Provider visibility check | `schema.rs` (`show_in_selector`) | `InputStatusRow.tsx:190-198, 251-259` (duplicated logic) |
| Model ID strings | N/A | `ai.ts:686-788` |
| Model display names | N/A | `models.ts:60-337, 343-702` |
| Credentials check | N/A | `ProviderSettings.tsx:47-104` (`getConfigured` functions) |
| Synthesis models | N/A | `AiSettings.tsx:239-294` (hardcoded, not from models.ts!) |

---

## Backend Duplication & Issues

### 1. Client Creation Logic Duplicated
The same client creation logic exists in **two places**:
- `create_*_components()` functions (lines 137-557 of `llm_client.rs`)
- `LlmClientFactory::create_client()` (lines 620-794 of `llm_client.rs`)

When adding a provider or fixing a bug, both must be updated.

### 2. Model Detection Logic Scattered
Model-specific behavior is detected in multiple places:

| Detection | Location | Purpose |
|-----------|----------|---------|
| `is_reasoning_model()` | `rig-openai-responses/src/lib.rs` | Route to correct OpenAI client |
| `model_supports_temperature()` | `model_capabilities.rs` | Skip temperature param |
| `openai_supports_web_search()` | `model_capabilities.rs` | Enable web search tool |
| `detect_thinking_history_support()` | `model_capabilities.rs` | Track reasoning in history |
| `VisionCapabilities::detect()` | `model_capabilities.rs` | Image upload support |

All use string prefix matching on model names.

### 3. No Central Model Registry
The backend has **no list of valid models**. It:
- Accepts any model string from frontend
- Detects capabilities via string matching
- Fails only at API call time if model is invalid

This means:
- No validation that a model exists for a provider
- Frontend is the only source of truth for available models
- Backend capability detection can drift from actual API behavior

### 4. Provider-Specific Hardcoding
Each provider has unique initialization logic:
- Vertex AI: Service account or ADC, extended thinking, web search
- OpenAI: 3 different client types based on model
- Ollama: No API key, custom base URL (ignored currently)
- Z.AI: Source channel, custom base URL

No abstraction - pure pattern matching on provider enum.

---

## Pain Points

### 1. Adding a New Provider Requires Changes In:
1. `backend/crates/qbit-settings/src/schema.rs` - Add enum variant + settings struct
2. `backend/crates/qbit-ai/src/llm_client.rs` - Add factory function
3. `frontend/lib/settings.ts` - Add TypeScript type + settings interface
4. `frontend/lib/ai.ts` - Add model constants
5. `frontend/lib/models.ts` - Add to PROVIDER_GROUPS + PROVIDER_GROUPS_NESTED
6. `frontend/components/Settings/ProviderSettings.tsx` - Add to PROVIDERS array

### 2. Adding a New Model Requires Changes In:
1. `frontend/lib/ai.ts` - Add constant
2. `frontend/lib/models.ts` - Add to both PROVIDER_GROUPS and PROVIDER_GROUPS_NESTED
3. Potentially `AiSettings.tsx` if it's for synthesis

### 3. Inconsistencies Found
- **Synthesis models are hardcoded separately** in `AiSettings.tsx` (lines 239-294) instead of sourcing from the central `models.ts`
- **Provider visibility logic duplicated** in `InputStatusRow.tsx` (appears twice: lines 190-198 and 251-259)
- **OpenRouter models hardcoded inline** in `models.ts` instead of having a constant like other providers

### 4. Type Safety Gaps
- Model IDs are plain strings - no compile-time check that a model belongs to a provider
- Backend accepts any model string - no validation

---

## Proposed Simplification Plan

### Phase 1: Backend Model Registry

**Goal**: Single source of truth for model metadata, eliminate string matching

```rust
// New: backend/crates/qbit-models/src/lib.rs

#[derive(Clone, Serialize)]
pub struct ModelDefinition {
    pub id: &'static str,
    pub display_name: &'static str,
    pub provider: AiProvider,
    pub capabilities: ModelCapabilities,
}

#[derive(Clone, Default, Serialize)]
pub struct ModelCapabilities {
    pub supports_temperature: bool,
    pub supports_thinking_history: bool,
    pub supports_vision: bool,
    pub supports_web_search: bool,
    pub is_reasoning_model: bool,  // Uses OpenAI reasoning client
    pub context_window: u32,
    pub max_output_tokens: u32,
}

pub static MODEL_REGISTRY: LazyLock<Vec<ModelDefinition>> = LazyLock::new(|| vec![
    ModelDefinition {
        id: "claude-opus-4-5@20251101",
        display_name: "Claude Opus 4.5",
        provider: AiProvider::VertexAi,
        capabilities: ModelCapabilities {
            supports_temperature: true,
            supports_thinking_history: true,
            supports_vision: true,
            supports_web_search: true,
            context_window: 200_000,
            ..Default::default()
        },
    },
    ModelDefinition {
        id: "gpt-5.2",
        display_name: "GPT 5.2",
        provider: AiProvider::Openai,
        capabilities: ModelCapabilities {
            supports_temperature: false,  // Reasoning model
            supports_thinking_history: true,
            supports_vision: true,
            supports_web_search: true,
            is_reasoning_model: true,
            context_window: 128_000,
            ..Default::default()
        },
    },
    // ...
]);

// Replace all string matching with registry lookup
pub fn get_model(id: &str) -> Option<&'static ModelDefinition> {
    MODEL_REGISTRY.iter().find(|m| m.id == id)
}

pub fn get_models_for_provider(provider: AiProvider) -> Vec<&'static ModelDefinition> {
    MODEL_REGISTRY.iter().filter(|m| m.provider == provider).collect()
}
```

**Benefits**:
- Eliminates `model_capabilities.rs` string matching
- Single place to update when models change
- Enables server-side validation
- Exposes to frontend via Tauri command

**Tauri command**:
```rust
#[tauri::command]
fn get_available_models(provider: Option<AiProvider>) -> Vec<ModelDefinition> {
    match provider {
        Some(p) => get_models_for_provider(p),
        None => MODEL_REGISTRY.clone(),
    }
}
```

**Frontend impact**: Can fetch models from backend instead of hardcoding

### Phase 2: Unify Backend Client Creation

**Goal**: Eliminate duplication between `create_*_components()` and `LlmClientFactory`

```rust
// Trait-based provider abstraction
pub trait LlmProvider: Send + Sync {
    fn provider_type(&self) -> AiProvider;
    fn create_client(&self, model: &str) -> Result<LlmClient>;
    fn validate_credentials(&self) -> Result<()>;
}

// Implementations
pub struct OpenAiProvider {
    api_key: String,
    enable_web_search: bool,
    web_search_context_size: String,
}

impl LlmProvider for OpenAiProvider {
    fn create_client(&self, model: &str) -> Result<LlmClient> {
        let model_def = get_model(model)
            .ok_or_else(|| anyhow!("Unknown model: {}", model))?;

        if model_def.capabilities.is_reasoning_model {
            // Use rig-openai-responses
            let client = rig_openai_responses::Client::new(&self.api_key);
            Ok(LlmClient::OpenAiReasoning(client.completion_model(model)))
        } else {
            // Use rig-core responses API
            let client = rig_openai::Client::new(&self.api_key)?;
            Ok(LlmClient::RigOpenAiResponses(client.completion_model(model)))
        }
    }
}

// Registry of providers (built from settings)
pub struct ProviderRegistry {
    providers: HashMap<AiProvider, Box<dyn LlmProvider>>,
}

impl ProviderRegistry {
    pub fn from_settings(settings: &QbitSettings) -> Self { ... }

    pub fn get(&self, provider: AiProvider) -> Option<&dyn LlmProvider> {
        self.providers.get(&provider).map(|p| p.as_ref())
    }
}
```

**Benefits**:
- Single client creation path
- Provider behavior encapsulated in trait impl
- Easier to add new providers
- `LlmClientFactory` becomes thin wrapper around `ProviderRegistry`

### Phase 3: Generate TypeScript Types from Rust

**Goal**: Eliminate manual sync between Rust and TypeScript

Options:
1. **ts-rs** crate - Generate TypeScript interfaces from Rust structs
2. **typeshare** crate - Generates types for multiple languages
3. **Manual codegen script** - Parse Rust and emit TypeScript

Recommended: `ts-rs` with a build step

```rust
// In schema.rs
use ts_rs::TS;

#[derive(TS)]
#[ts(export)]
pub enum AiProvider { ... }
```

### Phase 4: Frontend Consolidation

**Goal**: Single source of truth for all model-related data in `lib/models.ts`

Now that backend provides the model registry, frontend can fetch dynamically.

#### 4.1 Fetch models from backend
```typescript
// frontend/lib/models.ts
import { invoke } from "@tauri-apps/api/core";

export async function getAvailableModels(provider?: AiProvider): Promise<ModelDefinition[]> {
  return invoke("get_available_models", { provider });
}
```

#### 4.2 Derive PROVIDER_GROUPS from backend data
```typescript
// Single definition per provider (UI metadata only)
const PROVIDER_UI_CONFIG = {
  anthropic: {
    name: "Anthropic",
    icon: "🔶",
    description: "Direct Claude API access",
    checkConfigured: (s: AiSettings) => !!s.anthropic.api_key,
  },
  // ...
} as const;

// Models come from backend
export async function getProviderGroups(): Promise<ProviderGroup[]> {
  const models = await getAvailableModels();
  // Group by provider and merge with UI config
}
```

#### 4.3 Update synthesis models to use central source
```typescript
// In AiSettings.tsx
import { getProviderGroup } from "@/lib/models";

// Instead of hardcoded:
const vertexModels = await getProviderGroup("vertex_ai");
```

### Phase 5: Provider Plugin Architecture (Future)

**Note**: Only pursue this if adding many new providers becomes a common need.

**Goal**: Make adding providers fully modular

```
backend/crates/qbit-providers/
  src/
    traits.rs        # ProviderTrait definition
    anthropic.rs     # impl ProviderTrait for Anthropic
    openai.rs        # impl ProviderTrait for OpenAI
    registry.rs      # Provider discovery
```

Each provider module exports:
- Settings struct
- Model list
- LLM client factory
- UI metadata (name, icon, description)

---

## Implementation Priority

| Phase | Effort | Impact | Description |
|-------|--------|--------|-------------|
| **1 - Backend model registry** | Medium | High | Create `qbit-models` crate with `MODEL_REGISTRY`. Single source of truth for all model metadata. Eliminates string matching in `model_capabilities.rs`. |
| **2 - Unify client creation** | Medium | Medium | Consolidate duplicated logic in `create_*_components()` and `LlmClientFactory`. Use trait-based abstraction. |
| **3 - Type generation** | Low-Medium | Medium | Use `ts-rs` to generate TypeScript from Rust. Frontend fetches model list from backend via Tauri command. |
| **4 - Frontend consolidation** | Medium | High | Simplify frontend now that data comes from backend. Remove hardcoded model lists. |
| **5 - Plugin architecture** | High | High | Future work. Only if adding many providers. Full modular provider system. |

### Rationale

Starting with the backend (Phase 1) is critical because:
1. It establishes the **single source of truth** for model definitions
2. Frontend can then **fetch models dynamically** instead of hardcoding
3. Backend **validation** becomes possible (reject unknown models)
4. **Capability detection** moves from string matching to registry lookup
5. **Type generation** (Phase 3) can export the registry to TypeScript

---

## Quick Wins (Can Do Immediately)

### Frontend:
1. **Fix synthesis model lists** - Source from `models.ts` instead of hardcoding
2. **Extract visibility check logic** - Create `isProviderVisible(provider, settings)` helper
3. **Add OpenRouter constants** - Create `OPENROUTER_MODELS` like other providers
4. **DRY the provider metadata** - Single `PROVIDERS` definition used by both Settings and model selector

### Backend:
1. **Extract `is_reasoning_model()` to qbit-llm-providers** - Currently in rig-openai-responses, duplicated in model_capabilities.rs
2. **Add model validation** - Warn if model string doesn't match known patterns
3. **Consolidate capability detection** - Single `ModelCapabilities::detect(provider, model)` entry point
4. **Add tests for capability detection** - Already have some, but coverage is incomplete for edge cases

---

## Files to Modify

### Phase 1: Backend Model Registry
**New crate**: `backend/crates/qbit-models/`
- `src/lib.rs` - `ModelDefinition`, `ModelCapabilities`, `MODEL_REGISTRY`
- `src/registry.rs` - Lookup functions

**Modify**:
- `backend/crates/qbit-llm-providers/src/model_capabilities.rs` - Replace with registry lookups
- `backend/crates/qbit-ai/src/llm_client.rs` - Use registry for capability checks
- `backend/crates/qbit/src/ai/commands/*.rs` - Add `get_available_models` command
- `backend/Cargo.toml` - Add qbit-models to workspace

### Phase 2: Unify Client Creation
**New**: `backend/crates/qbit-llm-providers/src/provider_trait.rs`
- `LlmProvider` trait
- Per-provider implementations

**Modify**:
- `backend/crates/qbit-ai/src/llm_client.rs`
  - Remove duplicated logic in `create_*_components()` and `LlmClientFactory`
  - Both use `ProviderRegistry`
- `backend/crates/qbit-llm-providers/src/lib.rs` - Export provider trait

### Phase 3: Type Generation
- `backend/crates/qbit-settings/src/schema.rs` - Add ts-rs derives
- `backend/crates/qbit-models/src/lib.rs` - Add ts-rs derives
- `backend/crates/qbit-settings/Cargo.toml` - Add ts-rs dependency
- `backend/crates/qbit-models/Cargo.toml` - Add ts-rs dependency
- New: `scripts/generate-types.sh` - Build script to generate TypeScript

### Phase 4: Frontend Consolidation
- `frontend/lib/models.ts` - Fetch from backend, remove hardcoded lists
- `frontend/lib/ai.ts` - Remove model constants
- `frontend/lib/settings.ts` - Import generated types
- `frontend/components/Settings/ProviderSettings.tsx` - Use shared provider config
- `frontend/components/Settings/AiSettings.tsx` - Use models from backend
- `frontend/components/UnifiedInput/InputStatusRow.tsx` - Extract visibility logic

---

## Appendix: Current File Locations

```
Backend (Provider/Model Related):
├── backend/crates/qbit-settings/src/
│   └── schema.rs                    # AiProvider enum, per-provider settings structs
│
├── backend/crates/qbit-llm-providers/src/
│   ├── lib.rs                       # LlmClient enum (11 variants), ProviderConfig enum
│   ├── model_capabilities.rs        # ModelCapabilities, VisionCapabilities, string matching
│   └── openai_config.rs             # OpenAI web search config
│
├── backend/crates/qbit-ai/src/
│   └── llm_client.rs                # create_*_components() (10 functions), LlmClientFactory
│
├── backend/crates/rig-openai-responses/src/
│   └── lib.rs                       # is_reasoning_model(), custom OpenAI reasoning client
│
└── backend/crates/qbit/src/
    └── ai/commands/*.rs             # Tauri commands for AI initialization

Frontend:
├── frontend/lib/
│   ├── ai.ts                        # Model ID constants (8 objects), init functions
│   ├── models.ts                    # PROVIDER_GROUPS, PROVIDER_GROUPS_NESTED, helpers
│   └── settings.ts                  # TypeScript settings types (mirrors Rust schema.rs)
│
├── frontend/components/Settings/
│   ├── ProviderSettings.tsx         # Provider config UI, PROVIDERS array (duplicated)
│   ├── ModelSelector.tsx            # Model dropdown component
│   └── AiSettings.tsx               # Synthesis config (hardcoded models!)
│
└── frontend/components/UnifiedInput/
    └── InputStatusRow.tsx           # Footer model selector

User Configuration Files:
├── ~/.qbit/settings.toml            # Global provider/model settings
└── <workspace>/.qbit/project.toml   # Per-project provider/model override
```

---

## Summary: Adding a New Provider Today

To add a new provider (e.g., "Mistral"), you must modify:

### Backend (5 files):
1. `qbit-settings/src/schema.rs`:
   - Add `Mistral` variant to `AiProvider` enum
   - Add `MistralSettings` struct
   - Add `mistral` field to `AiSettings`
   - Update `Default` impl

2. `qbit-llm-providers/src/lib.rs`:
   - Add `RigMistral(...)` variant to `LlmClient` enum
   - Add `MistralClientConfig` struct
   - Add `Mistral { ... }` variant to `ProviderConfig` enum
   - Update all `match` statements

3. `qbit-llm-providers/src/model_capabilities.rs`:
   - Add `"mistral"` cases to capability detection functions

4. `qbit-ai/src/llm_client.rs`:
   - Add `create_mistral_components()` function
   - Add `AiProvider::Mistral` case to `LlmClientFactory::create_client()`

5. `Cargo.toml` (if using a new rig provider crate):
   - Add dependency

### Frontend (5 files):
1. `lib/settings.ts`:
   - Add `"mistral"` to `AiProvider` type union
   - Add `MistralSettings` interface
   - Add `mistral` field to `AiSettings` interface
   - Update `DEFAULT_SETTINGS`

2. `lib/ai.ts`:
   - Add `MISTRAL_MODELS` constant

3. `lib/models.ts`:
   - Add Mistral to `PROVIDER_GROUPS`
   - Add Mistral to `PROVIDER_GROUPS_NESTED`

4. `components/Settings/ProviderSettings.tsx`:
   - Add Mistral to `PROVIDERS` array
   - Add Mistral-specific form fields

5. `components/UnifiedInput/InputStatusRow.tsx`:
   - Add `mistral: settings.ai.mistral.show_in_selector` (appears twice!)

**Total: 10 files, ~20 separate changes**
