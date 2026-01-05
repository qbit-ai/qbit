# Image Handling

Multi-modal image attachment support for Qbit, enabling users to send images alongside text prompts to vision-capable AI models.

## Overview

This feature allows users to attach images to their prompts when using AI providers that support vision capabilities. The implementation follows patterns from the Vercel AI SDK and includes:

- **Provider capability detection** - Automatically detects if the current model supports images
- **Frontend attachment UI** - Image picker with preview, validation, and removal
- **Graceful degradation** - Strips images and warns users when sending to non-vision models
- **Multi-part prompts** - Supports sending text and multiple images in a single prompt

## Supported Providers

| Provider | Vision Support | Max Image Size | Supported Formats |
|----------|---------------|----------------|-------------------|
| Anthropic (Direct) | Claude 3+ models | 5MB | PNG, JPEG, GIF, WebP |
| Vertex AI (Anthropic) | Claude 3+ models | 5MB | PNG, JPEG, GIF, WebP |
| OpenAI | GPT-4+, o1+, o3+ | 20MB | PNG, JPEG, GIF, WebP |
| Gemini | All models | 20MB | PNG, JPEG, GIF, WebP |
| Ollama | No | - | - |
| Groq | No | - | - |
| xAI | No | - | - |
| Z.AI | No | - | - |
| OpenRouter | No* | - | - |

*OpenRouter support depends on the underlying model but is not currently implemented.

## Architecture

### Data Flow

```
Frontend (ImageAttachment)
    |
    | File → base64 data URL
    v
Frontend (UnifiedInput)
    |
    | PromptPayload { parts: [TextPart, ImagePart, ...] }
    v
Tauri Command (send_ai_prompt_with_attachments)
    |
    | Validate payload against VisionCapabilities
    | Convert PromptPart → rig::message::UserContent
    v
AgentBridge.execute_with_content()
    |
    | Message::User { content: Vec<UserContent> }
    v
LLM Provider (rig-anthropic-vertex, etc.)
```

### Key Types

#### Backend (Rust)

```rust
// qbit-core/src/message.rs
pub enum PromptPart {
    Text { text: String },
    Image {
        data: String,           // Base64 or data URL
        media_type: Option<String>,
        filename: Option<String>,
    },
}

pub struct PromptPayload {
    pub parts: Vec<PromptPart>,
}
```

```rust
// qbit-llm-providers/src/model_capabilities.rs
pub struct VisionCapabilities {
    pub supports_vision: bool,
    pub max_image_size_bytes: usize,
    pub supported_formats: Vec<String>,
}
```

#### Frontend (TypeScript)

```typescript
// frontend/lib/ai.ts
interface TextPart {
  type: "text";
  text: string;
}

interface ImagePart {
  type: "image";
  data: string;        // Base64 data URL
  media_type?: string;
  filename?: string;
}

type PromptPart = TextPart | ImagePart;

interface PromptPayload {
  parts: PromptPart[];
}

interface VisionCapabilities {
  supports_vision: boolean;
  max_image_size_bytes: number;
  supported_formats: string[];
}
```

## Frontend Implementation

### ImageAttachment Component

Location: `frontend/components/UnifiedInput/ImageAttachment.tsx`

A React component that provides:
- Attach button (hidden when vision not supported)
- File input with proper MIME type filtering
- Image preview thumbnails
- Remove button on hover
- Validation against provider capabilities

```tsx
<ImageAttachment
  attachments={attachments}
  onAttachmentsChange={setAttachments}
  capabilities={visionCapabilities}
  disabled={isLoading}
/>
```

### Integration with UnifiedInput

The UnifiedInput component:
1. Fetches vision capabilities on mount via `getVisionCapabilities(sessionId)`
2. Manages attachment state with `useState<ImagePart[]>`
3. On submit, builds a `PromptPayload` with text and image parts
4. Calls `sendPromptWithAttachments(sessionId, payload)` instead of `sendPromptSession()`

### Warning Event Handling

When images are sent to a non-vision model, the backend emits a `warning` event:

```typescript
// In useAiEvents.ts
case "warning":
  // Display warning toast or notification
  toast.warning(event.message);
  break;
```

## Backend Implementation

### Tauri Commands

Location: `backend/crates/qbit/src/ai/commands/core.rs`

**`get_vision_capabilities`**
```rust
#[tauri::command]
pub async fn get_vision_capabilities(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<VisionCapabilities, String>
```
Returns vision capabilities for the current session's provider/model.

**`send_ai_prompt_with_attachments`**
```rust
#[tauri::command]
pub async fn send_ai_prompt_with_attachments(
    state: State<'_, AppState>,
    session_id: String,
    payload: PromptPayload,
) -> Result<String, String>
```
Sends a multi-modal prompt. If the provider doesn't support vision:
1. Strips image parts from the payload
2. Emits a `Warning` event to the frontend
3. Sends text-only to the model

### AgentBridge Methods

Location: `backend/crates/qbit-ai/src/agent_bridge.rs`

**`execute_with_content`**
```rust
pub async fn execute_with_content(
    &self,
    content: Vec<UserContent>
) -> Result<String>
```
Executes an agent turn with rich content (text + images). Delegates to the agentic loop with the multi-part user message.

### Provider Image Conversion

**Anthropic Vertex** (`rig-anthropic-vertex/src/completion.rs`):
Images are converted to Anthropic's `ContentBlock::Image` format with base64 source.

**Other Providers**: Support varies. The frontend capability check prevents users from attaching images for unsupported providers.

## API Reference

### Frontend Functions

```typescript
// Get vision capabilities for a session
async function getVisionCapabilities(sessionId: string): Promise<VisionCapabilities>

// Send a multi-modal prompt
async function sendPromptWithAttachments(
  sessionId: string,
  payload: PromptPayload
): Promise<string>

// Helper: create text-only payload
function createTextPayload(text: string): PromptPayload

// Helper: check if payload has images
function hasImages(payload: PromptPayload): boolean

// Helper: extract text from payload
function extractText(payload: PromptPayload): string
```

### AI Events

```typescript
// Warning event (emitted when images stripped)
{
  type: "warning",
  message: string,
  session_id: string
}
```

## Usage Example

### Sending an Image with Text

```typescript
import {
  getVisionCapabilities,
  sendPromptWithAttachments,
  type PromptPayload,
  type ImagePart
} from "@/lib/ai";

// Check if vision is supported
const caps = await getVisionCapabilities(sessionId);
if (!caps.supports_vision) {
  console.warn("Current model doesn't support images");
  return;
}

// Build payload with text and image
const payload: PromptPayload = {
  parts: [
    { type: "text", text: "What's in this image?" },
    {
      type: "image",
      data: "data:image/png;base64,iVBORw0KGgo...",
      media_type: "image/png",
      filename: "screenshot.png"
    }
  ]
};

// Send to AI
const response = await sendPromptWithAttachments(sessionId, payload);
```

### Reading a File as ImagePart

```typescript
async function fileToImagePart(file: File): Promise<ImagePart> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => {
      resolve({
        type: "image",
        data: reader.result as string,
        media_type: file.type,
        filename: file.name,
      });
    };
    reader.onerror = reject;
    reader.readAsDataURL(file);
  });
}
```

## Validation

The payload is validated before sending:

1. **Provider Support**: Check `supports_vision` flag
2. **MIME Type**: Must be in `supported_formats` list
3. **File Size**: Must be under `max_image_size_bytes`

Validation errors are returned from the Tauri command as error strings.

```rust
// Example validation in PromptPayload::validate()
if estimated_bytes > max_image_size_bytes {
    return Err(format!(
        "Image too large: {}MB (max {}MB)",
        estimated_bytes / 1024 / 1024,
        max_image_size_bytes / 1024 / 1024
    ));
}
```

## Testing

### Unit Tests (Rust)

Location: `backend/crates/qbit-core/src/message.rs`

- `test_from_text` - PromptPayload from plain text
- `test_has_images` - Detect images in payload
- `test_validate_unsupported_format` - Reject invalid MIME types
- `test_serde_roundtrip` - Serialization/deserialization
- `test_deserialize_from_frontend_format` - Parse exact frontend JSON format

Location: `backend/crates/qbit-llm-providers/src/model_capabilities.rs`

- Vision capability detection for all providers
- Model name pattern matching

### Frontend Tests

- ImageAttachment component rendering
- File validation (size, type)
- Attachment add/remove functionality
- Capability-based UI state (disabled when no vision)

## Future Enhancements

1. **Session Persistence** - Save images to `~/.qbit/sessions/<session-id>/assets/` and reference by UUID
2. **Clipboard Paste** - Support pasting images directly from clipboard
3. **Drag and Drop** - Drop images onto the input area
4. **Image Preview Modal** - Click to view full-size image
5. **OpenRouter Vision** - Detect vision support based on underlying model
