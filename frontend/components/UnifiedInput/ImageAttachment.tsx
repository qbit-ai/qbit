import { ImagePlus, X } from "lucide-react";
import { useCallback, useRef } from "react";
import type { ImagePart, VisionCapabilities } from "@/lib/ai";
import { logger } from "@/lib/logger";
import { cn } from "@/lib/utils";

interface ImageAttachmentProps {
  /** Currently attached images */
  attachments: ImagePart[];
  /** Callback when attachments change */
  onAttachmentsChange: (attachments: ImagePart[]) => void;
  /** Vision capabilities for the current provider/model */
  capabilities: VisionCapabilities | null;
  /** Whether the attachment button should be disabled */
  disabled?: boolean;
}

/**
 * Image attachment component for the UnifiedInput.
 * Provides a button to attach images and displays previews of attached images.
 */
export function ImageAttachment({
  attachments,
  onAttachmentsChange,
  capabilities,
  disabled = false,
}: ImageAttachmentProps) {
  const fileInputRef = useRef<HTMLInputElement>(null);

  const supportsVision = capabilities?.supports_vision ?? false;
  const hasAttachments = attachments.length > 0;

  const handleButtonClick = useCallback(() => {
    fileInputRef.current?.click();
  }, []);

  const handleFileChange = useCallback(
    async (e: React.ChangeEvent<HTMLInputElement>) => {
      const files = e.target.files;
      if (!files || files.length === 0 || !capabilities) return;

      const newAttachments: ImagePart[] = [];

      for (const file of files) {
        // Validate file type
        if (!capabilities.supported_formats.includes(file.type)) {
          logger.warn(`Unsupported image type: ${file.type}`);
          continue;
        }

        // Validate file size
        if (file.size > capabilities.max_image_size_bytes) {
          logger.warn(
            `Image too large: ${(file.size / 1024 / 1024).toFixed(1)}MB (max ${(capabilities.max_image_size_bytes / 1024 / 1024).toFixed(0)}MB)`
          );
          continue;
        }

        // Read file as base64
        const base64 = await readFileAsBase64(file);
        newAttachments.push({
          type: "image",
          data: base64,
          media_type: file.type,
          filename: file.name,
        });
      }

      if (newAttachments.length > 0) {
        onAttachmentsChange([...attachments, ...newAttachments]);
      }

      // Reset input so the same file can be selected again
      e.target.value = "";
    },
    [attachments, capabilities, onAttachmentsChange]
  );

  const handleRemove = useCallback(
    (index: number) => {
      onAttachmentsChange(attachments.filter((_, i) => i !== index));
    },
    [attachments, onAttachmentsChange]
  );

  // Show if vision is supported OR if there are attachments (for removal)
  if (!supportsVision && !hasAttachments) {
    return null;
  }

  const acceptFormats = capabilities?.supported_formats.join(",") ?? "image/*";

  return (
    <div className="flex items-center gap-2">
      {/* Attachment button - only show if vision is supported */}
      {supportsVision && (
        <button
          type="button"
          onClick={handleButtonClick}
          disabled={disabled}
          className={cn(
            "h-7 w-7 flex items-center justify-center rounded-md shrink-0",
            "transition-all duration-150",
            "text-muted-foreground hover:text-foreground hover:bg-muted",
            disabled && "opacity-50 cursor-not-allowed"
          )}
          title="Attach image"
        >
          <ImagePlus className="w-4 h-4" />
        </button>
      )}

      {/* Hidden file input */}
      <input
        ref={fileInputRef}
        type="file"
        accept={acceptFormats}
        multiple
        onChange={handleFileChange}
        className="hidden"
      />

      {/* Image previews - always shown when attachments exist */}
      {hasAttachments && (
        <div className="flex items-center gap-1.5">
          {attachments.map((attachment, index) => (
            <div key={`${attachment.filename}-${index}`} className="relative group">
              <img
                src={attachment.data}
                alt={attachment.filename || `Image ${index + 1}`}
                className="h-8 w-8 rounded object-cover border border-[var(--color-border-medium)]"
              />
              <button
                type="button"
                onClick={() => handleRemove(index)}
                className={cn(
                  "absolute -top-1.5 -right-1.5",
                  "h-4 w-4 rounded-full",
                  "bg-destructive text-destructive-foreground",
                  "flex items-center justify-center",
                  "opacity-0 group-hover:opacity-100 transition-opacity",
                  "hover:bg-destructive/90"
                )}
                title="Remove image"
              >
                <X className="w-2.5 h-2.5" />
              </button>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

/**
 * Read a file as a base64 data URL.
 */
export function readFileAsBase64(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => {
      if (typeof reader.result === "string") {
        resolve(reader.result);
      } else {
        reject(new Error("Failed to read file as base64"));
      }
    };
    reader.onerror = () => reject(reader.error);
    reader.readAsDataURL(file);
  });
}
