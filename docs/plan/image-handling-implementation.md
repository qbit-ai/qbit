# Image Handling Implementation

## Objective

Implement multi-modal image attachment support for qbit, following Vercel AI SDK patterns. Support vision-capable providers (OpenAI, Anthropic, Anthropic Vertex, Gemini) with graceful error handling for unsupported providers.

---

## Phase 1: Provider Vision Capability Infrastructure

### 1.1 Add Vision Capability to Provider Config

**File:** `backend/crates/qbit-llm-providers/src/lib.rs`

Add a `supports_vision` field to the provider configuration:

```rust
pub struct ProviderCapabilities {
    pub supports_vision: bool,
    pub max_image_size_bytes: usize,  // Provider-specific limits
    pub supported_image_types: Vec<String>,
}

impl ProviderCapabilities {
    pub fn for_provider(provider: &str, model: &str) -> Self {
        match provider {
            "vertex_ai" | "anthropic" => Self {
                supports_vision: model.contains("claude-3") || model.contains("claude-4"),
                max_image_size_bytes: 5 * 1024 * 1024,  // 5MB
                supported_image_types: vec!["image/png", "image/jpeg", "image/gif", "image/webp"].into_iter().map(String::from).collect(),
            },
            "openai" => Self {
                supports_vision: model.contains("gpt-4") || model.contains("o1") || model.contains("o3"),
                max_image_size_bytes: 20 * 1024 * 1024,  // 20MB
                supported_image_types: vec!["image/png", "image/jpeg", "image/gif", "image/webp"].into_iter().map(String::from).collect(),
            },
            "gemini" => Self {
                supports_vision: true,  // All Gemini models support vision
                max_image_size_bytes: 20 * 1024 * 1024,
                supported_image_types: vec!["image/png", "image/jpeg", "image/gif", "image/webp"].into_iter().map(String::from).collect(),
            },
            // Unsupported providers
            "ollama" | "groq" | "xai" | "zai" | "openrouter" => Self {
                supports_vision: false,
                max_image_size_bytes: 0,
                supported_image_types: vec![],
            },
            _ => Self {
                supports_vision: false,
                max_image_size_bytes: 0,
                supported_image_types: vec![],
            },
        }
    }
}
```

### 1.2 Expose Capability Check via Tauri Command

**File:** `backend/crates/qbit/src/ai/commands/core.rs`

```rust
#[tauri::command]
pub async fn get_provider_capabilities(
    state: State<'_, AppState>,
) -> Result<ProviderCapabilities, String> {
    let settings = state.settings.read().await;
    let provider = &settings.ai.provider;
    let model = &settings.ai.model;
    Ok(ProviderCapabilities::for_provider(provider, model))
}
```

---

## Phase 2: Fix Anthropic Vertex Image Conversion (Critical Bug)

### 2.1 Wire Up Image Conversion

**File:** `backend/crates/rig-anthropic-vertex/src/completion.rs`

The current code at line ~83 drops images with `_ => None`. Fix this:

```rust
fn convert_message(msg: &Message) -> types::Message {
    match msg {
        Message::User { content } => {
            let blocks: Vec<ContentBlock> = content
                .iter()
                .filter_map(|c| {
                    use rig::message::UserContent;
                    match c {
                        UserContent::Text(text) => Some(ContentBlock::Text {
                            text: text.text.clone(),
                        }),
                        UserContent::Image(image) => {
                            // Convert rig image to Anthropic ContentBlock
                            Some(ContentBlock::Image {
                                source: ImageSource {
                                    source_type: "base64".to_string(),
                                    media_type: image.media_type.clone()
                                        .unwrap_or_else(|| "image/png".to_string()),
                                    data: image.data.clone(),
                                },
                            })
                        },
                        UserContent::ToolResult(result) => Some(ContentBlock::ToolResult {
                            tool_use_id: result.id.clone(),
                            content: result.content.clone(),
                        }),
                        _ => None,  // Skip unsupported content types
                    }
                })
                .collect();
            // ... rest of conversion
        }
        // ... other cases
    }
}
```

### 2.2 Add Image Conversion for Other Providers

**File:** `backend/crates/qbit-ai/src/llm_client.rs`

Ensure each vision-capable provider has image conversion:

- **OpenAI:** Map to `{ type: "image_url", image_url: { url: "data:image/png;base64,..." } }`
- **Gemini:** Map to `{ inlineData: { mimeType: "...", data: "..." } }`
- **Anthropic Direct:** Same as Vertex

---

## Phase 3: Backend Prompt Payload

### 3.1 Define PromptPayload Struct

**File:** `backend/crates/qbit-core/src/prompt.rs` (new file)

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PromptPart {
    Text { text: String },
    Image {
        data: String,           // Base64 or data URL
        media_type: Option<String>,
        filename: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptPayload {
    pub parts: Vec<PromptPart>,
}

impl PromptPayload {
    /// Create from plain text (backward compatibility)
    pub fn from_text(text: String) -> Self {
        Self {
            parts: vec![PromptPart::Text { text }],
        }
    }

    /// Extract text content only (for unsupported providers)
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

    /// Check if payload contains images
    pub fn has_images(&self) -> bool {
        self.parts.iter().any(|p| matches!(p, PromptPart::Image { .. }))
    }

    /// Validate images against provider capabilities
    pub fn validate(&self, capabilities: &ProviderCapabilities) -> Result<(), String> {
        for part in &self.parts {
            if let PromptPart::Image { data, media_type, .. } = part {
                // Check if provider supports vision
                if !capabilities.supports_vision {
                    return Err("Current provider does not support image attachments".to_string());
                }

                // Check MIME type
                let mime = media_type.as_deref().unwrap_or("image/png");
                if !capabilities.supported_image_types.iter().any(|t| t == mime) {
                    return Err(format!("Unsupported image type: {}", mime));
                }

                // Check size (base64 has ~33% overhead, so actual bytes ≈ len * 3/4)
                let estimated_bytes = data.len() * 3 / 4;
                if estimated_bytes > capabilities.max_image_size_bytes {
                    return Err(format!(
                        "Image too large: {}MB (max {}MB)",
                        estimated_bytes / 1024 / 1024,
                        capabilities.max_image_size_bytes / 1024 / 1024
                    ));
                }
            }
        }
        Ok(())
    }
}
```

### 3.2 Add New Tauri Command (Backward Compatible)

**File:** `backend/crates/qbit/src/ai/commands/core.rs`

Keep the existing `send_ai_prompt_session` for compatibility, add a new command:

```rust
#[tauri::command]
pub async fn send_ai_prompt_with_attachments(
    app: AppHandle,
    state: State<'_, AppState>,
    session_id: String,
    payload: PromptPayload,
) -> Result<String, String> {
    // Get provider capabilities
    let settings = state.settings.read().await;
    let capabilities = ProviderCapabilities::for_provider(&settings.ai.provider, &settings.ai.model);

    // Validate payload
    payload.validate(&capabilities).map_err(|e| e)?;

    // If provider doesn't support vision but payload has images, warn and use text-only
    let effective_payload = if payload.has_images() && !capabilities.supports_vision {
        tracing::warn!("Provider doesn't support images, sending text-only");
        // Emit warning event to frontend
        app.emit("ai-event", AiEvent::Warning {
            message: "Images removed: current model doesn't support vision".to_string(),
        }).ok();
        PromptPayload::from_text(payload.text_only())
    } else {
        payload
    };

    // Convert to UserContent parts
    let content_parts: Vec<UserContent> = effective_payload.parts
        .into_iter()
        .map(|p| match p {
            PromptPart::Text { text } => UserContent::Text(text.into()),
            PromptPart::Image { data, media_type, .. } => {
                // Strip data URL prefix if present
                let base64_data = if data.starts_with("data:") {
                    data.split(',').nth(1).unwrap_or(&data).to_string()
                } else {
                    data
                };
                UserContent::Image(ImageContent {
                    data: base64_data,
                    media_type,
                })
            }
        })
        .collect();

    // Continue with existing agent flow...
    send_prompt_internal(app, state, session_id, content_parts).await
}
```

### 3.3 Update Agent Bridge

**File:** `backend/crates/qbit-ai/src/agent_bridge.rs`

Modify `build_user_message` to accept `Vec<UserContent>` instead of `String`:

```rust
pub fn build_user_message(content_parts: Vec<UserContent>) -> Message {
    Message::User { content: content_parts }
}
```

---

## Phase 4: Frontend Implementation

### 4.1 TypeScript Types

**File:** `frontend/lib/ai.ts`

```typescript
// Prompt payload types
export interface TextPart {
  type: "text";
  text: string;
}

export interface ImagePart {
  type: "image";
  data: string;        // Base64 or data URL
  mediaType?: string;
  filename?: string;
}

export type PromptPart = TextPart | ImagePart;

export interface PromptPayload {
  parts: PromptPart[];
}

export interface ProviderCapabilities {
  supports_vision: boolean;
  max_image_size_bytes: number;
  supported_image_types: string[];
}

// API wrappers
export async function getProviderCapabilities(): Promise<ProviderCapabilities> {
  return invoke("get_provider_capabilities");
}

export async function sendPromptWithAttachments(
  sessionId: string,
  payload: PromptPayload
): Promise<string> {
  return invoke("send_ai_prompt_with_attachments", { sessionId, payload });
}

// Helper: Convert File to ImagePart
export async function fileToImagePart(file: File): Promise<ImagePart> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => {
      const dataUrl = reader.result as string;
      resolve({
        type: "image",
        data: dataUrl,
        mediaType: file.type,
        filename: file.name,
      });
    };
    reader.onerror = () => reject(new Error("Failed to read file"));
    reader.readAsDataURL(file);
  });
}
```

### 4.2 Attachment UI Component

**File:** `frontend/components/UnifiedInput/ImageAttachment.tsx` (new file)

```tsx
import { useState, useRef, useCallback } from "react";
import { X, Image as ImageIcon, AlertCircle } from "lucide-react";
import { ImagePart, ProviderCapabilities } from "@/lib/ai";
import { Button } from "@/components/ui/button";

interface ImageAttachmentProps {
  attachments: ImagePart[];
  onAttachmentsChange: (attachments: ImagePart[]) => void;
  capabilities: ProviderCapabilities | null;
  disabled?: boolean;
}

export function ImageAttachment({
  attachments,
  onAttachmentsChange,
  capabilities,
  disabled,
}: ImageAttachmentProps) {
  const inputRef = useRef<HTMLInputElement>(null);
  const [error, setError] = useState<string | null>(null);

  const validateFile = useCallback(
    (file: File): string | null => {
      if (!capabilities) return "Loading provider capabilities...";
      if (!capabilities.supports_vision) {
        return "Current model doesn't support images";
      }
      if (!capabilities.supported_image_types.includes(file.type)) {
        return `Unsupported type: ${file.type}`;
      }
      if (file.size > capabilities.max_image_size_bytes) {
        const maxMB = capabilities.max_image_size_bytes / 1024 / 1024;
        return `Image too large (max ${maxMB}MB)`;
      }
      return null;
    },
    [capabilities]
  );

  const handleFileSelect = useCallback(
    async (files: FileList | null) => {
      if (!files || files.length === 0) return;
      setError(null);

      const newAttachments: ImagePart[] = [];
      for (const file of Array.from(files)) {
        const validationError = validateFile(file);
        if (validationError) {
          setError(validationError);
          continue;
        }

        try {
          const reader = new FileReader();
          const dataUrl = await new Promise<string>((resolve, reject) => {
            reader.onload = () => resolve(reader.result as string);
            reader.onerror = reject;
            reader.readAsDataURL(file);
          });

          newAttachments.push({
            type: "image",
            data: dataUrl,
            mediaType: file.type,
            filename: file.name,
          });
        } catch {
          setError("Failed to read image file");
        }
      }

      if (newAttachments.length > 0) {
        onAttachmentsChange([...attachments, ...newAttachments]);
      }
    },
    [attachments, onAttachmentsChange, validateFile]
  );

  const removeAttachment = useCallback(
    (index: number) => {
      onAttachmentsChange(attachments.filter((_, i) => i !== index));
    },
    [attachments, onAttachmentsChange]
  );

  const isVisionSupported = capabilities?.supports_vision ?? false;

  return (
    <div className="flex flex-col gap-2">
      {/* Attachment button */}
      <Button
        type="button"
        variant="ghost"
        size="sm"
        disabled={disabled || !isVisionSupported}
        onClick={() => inputRef.current?.click()}
        title={isVisionSupported ? "Attach image" : "Current model doesn't support images"}
      >
        <ImageIcon className="h-4 w-4" />
      </Button>

      <input
        ref={inputRef}
        type="file"
        accept="image/png,image/jpeg,image/gif,image/webp"
        multiple
        className="hidden"
        onChange={(e) => handleFileSelect(e.target.files)}
      />

      {/* Error display */}
      {error && (
        <div className="flex items-center gap-2 text-sm text-destructive">
          <AlertCircle className="h-4 w-4" />
          {error}
        </div>
      )}

      {/* Attachment previews */}
      {attachments.length > 0 && (
        <div className="flex flex-wrap gap-2">
          {attachments.map((attachment, index) => (
            <div
              key={index}
              className="relative group rounded-md overflow-hidden border"
            >
              <img
                src={attachment.data}
                alt={attachment.filename ?? "Attachment"}
                className="h-16 w-16 object-cover"
              />
              <button
                type="button"
                onClick={() => removeAttachment(index)}
                className="absolute top-0 right-0 p-1 bg-black/50 text-white opacity-0 group-hover:opacity-100 transition-opacity"
              >
                <X className="h-3 w-3" />
              </button>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
```

### 4.3 Integrate into UnifiedInput

**File:** `frontend/components/UnifiedInput/UnifiedInput.tsx`

```tsx
// Add imports
import { ImageAttachment } from "./ImageAttachment";
import {
  ImagePart,
  PromptPayload,
  getProviderCapabilities,
  sendPromptWithAttachments,
  ProviderCapabilities,
} from "@/lib/ai";

// Inside component
const [attachments, setAttachments] = useState<ImagePart[]>([]);
const [capabilities, setCapabilities] = useState<ProviderCapabilities | null>(null);

// Fetch capabilities on mount and when provider changes
useEffect(() => {
  getProviderCapabilities().then(setCapabilities).catch(console.error);
}, [/* provider dependency */]);

// Modify submit handler
const handleSubmit = async () => {
  if (!inputValue.trim() && attachments.length === 0) return;

  const payload: PromptPayload = {
    parts: [
      // Text part (if any)
      ...(inputValue.trim() ? [{ type: "text" as const, text: inputValue }] : []),
      // Image parts
      ...attachments,
    ],
  };

  try {
    await sendPromptWithAttachments(sessionId, payload);
    setInputValue("");
    setAttachments([]);
  } catch (error) {
    // Handle error
  }
};

// In JSX, add ImageAttachment component near input
<ImageAttachment
  attachments={attachments}
  onAttachmentsChange={setAttachments}
  capabilities={capabilities}
  disabled={isLoading}
/>
```

---

## Phase 5: Error Handling for Unsupported Providers

### 5.1 Frontend Validation

Before allowing attachment, check capabilities:

```typescript
// In ImageAttachment component
if (!capabilities?.supports_vision) {
  // Disable attachment button
  // Show tooltip: "Current model doesn't support images"
}
```

### 5.2 Backend Graceful Degradation

When images are sent to unsupported provider:

```rust
// In send_ai_prompt_with_attachments
if payload.has_images() && !capabilities.supports_vision {
    // Option 1: Reject with clear error
    return Err("Cannot send images: current model doesn't support vision. Please switch to Claude, GPT-4, or Gemini.".to_string());

    // Option 2: Strip images and warn (alternative approach)
    // app.emit("ai-event", AiEvent::Warning { message: "..." });
    // let payload = PromptPayload::from_text(payload.text_only());
}
```

### 5.3 Warning Event for Degradation

**File:** `backend/crates/qbit-core/src/events.rs`

Add warning variant if not exists:

```rust
#[derive(Clone, Serialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AiEvent {
    // ... existing variants
    Warning {
        message: String,
    },
}
```

---

## Phase 6: Session Persistence (Optional, Phase 2)

### 6.1 Asset Storage Structure

```
~/.qbit/sessions/
  <session-id>/
    session.json       # Main session file
    assets/
      <uuid>.png       # Stored images
      <uuid>.jpg
```

### 6.2 Session Message with Attachments

**File:** `backend/crates/qbit-core/src/session/message.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachmentRef {
    pub id: String,           // UUID
    pub media_type: String,
    pub filename: Option<String>,
    pub asset_path: String,   // Relative path: "assets/<uuid>.png"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QbitSessionMessage {
    pub role: String,
    pub content: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attachments: Vec<AttachmentRef>,
}
```

### 6.3 Path Sanitization

```rust
fn sanitize_filename(filename: &str) -> String {
    filename
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '.' || *c == '-' || *c == '_')
        .collect::<String>()
        .chars()
        .take(100)
        .collect()
}

fn save_asset(session_dir: &Path, data: &[u8], media_type: &str) -> Result<String> {
    let assets_dir = session_dir.join("assets");
    std::fs::create_dir_all(&assets_dir)?;

    let ext = match media_type {
        "image/png" => "png",
        "image/jpeg" => "jpg",
        "image/gif" => "gif",
        "image/webp" => "webp",
        _ => "bin",
    };

    let id = uuid::Uuid::new_v4().to_string();
    let filename = format!("{}.{}", id, ext);
    let path = assets_dir.join(&filename);

    // Verify path is within assets directory (prevent traversal)
    let canonical = path.canonicalize().unwrap_or(path.clone());
    if !canonical.starts_with(&assets_dir) {
        return Err(anyhow::anyhow!("Invalid asset path"));
    }

    std::fs::write(&path, data)?;
    Ok(format!("assets/{}", filename))
}
```

---

## Testing Checklist

### Unit Tests (Rust)

- [ ] `ProviderCapabilities::for_provider` returns correct values
- [ ] `PromptPayload::validate` rejects oversized images
- [ ] `PromptPayload::validate` rejects unsupported MIME types
- [ ] `PromptPayload::text_only` extracts text correctly
- [ ] Image conversion in `rig-anthropic-vertex` produces valid ContentBlock
- [ ] Path sanitization prevents directory traversal

### Integration Tests (Rust)

- [ ] `send_ai_prompt_with_attachments` with valid image succeeds
- [ ] `send_ai_prompt_with_attachments` with unsupported provider returns error
- [ ] Session save/restore preserves attachment references

### Frontend Tests (Vitest)

- [ ] `fileToImagePart` converts File to correct format
- [ ] ImageAttachment validates file size
- [ ] ImageAttachment validates MIME type
- [ ] Attachment button disabled when vision not supported
- [ ] Preview renders correctly
- [ ] Remove button works

### E2E Tests (Playwright)

- [ ] Attach PNG and send to Claude
- [ ] Attach JPEG and send to GPT-4
- [ ] Attachment rejected for Ollama model
- [ ] Oversized image shows error
- [ ] Multiple images can be attached

---

## Implementation Order

1. **Phase 1:** Provider capabilities (backend only, no breaking changes)
2. **Phase 2:** Fix Anthropic Vertex conversion (critical bug fix)
3. **Phase 3:** Backend payload handling + new command
4. **Phase 4:** Frontend UI + integration
5. **Phase 5:** Error handling + testing
6. **Phase 6:** Session persistence (optional, defer if needed)

---

## Success Criteria

- [ ] Images can be attached and sent to Claude (Vertex/Direct)
- [ ] Images can be attached and sent to GPT-4
- [ ] Images can be attached and sent to Gemini
- [ ] Clear error message when using unsupported provider
- [ ] Images too large are rejected with helpful message
- [ ] Unsupported file types are rejected
- [ ] UI is disabled/hidden when provider doesn't support vision
