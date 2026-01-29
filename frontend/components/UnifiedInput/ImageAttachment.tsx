import { ImagePlus, X } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import type { ImagePart, VisionCapabilities } from "@/lib/ai";
import { logger } from "@/lib/logger";
import { cn } from "@/lib/utils";

// CSS for animations
const animationKeyframes = `
@keyframes pop-in {
  0% {
    opacity: 0;
    transform: scale(0.8);
  }
  100% {
    opacity: 1;
    transform: scale(1);
  }
}

@keyframes pop-out {
  0% {
    opacity: 1;
    transform: scale(1);
  }
  100% {
    opacity: 0;
    transform: scale(0.5);
  }
}
`;

interface ExitingImage {
  attachment: ImagePart;
  key: string;
}

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
  // Track images that are animating out (for exit animation)
  const [exitingImages, setExitingImages] = useState<ExitingImage[]>([]);
  // Track which images are newly added (for pop-in animation)
  const [enteringIndices, setEnteringIndices] = useState<Set<number>>(new Set());
  const prevAttachmentsRef = useRef<ImagePart[]>([]);

  const supportsVision = capabilities?.supports_vision ?? false;
  const hasAttachments = attachments.length > 0;
  const hasExitingImages = exitingImages.length > 0;

  // Track changes to trigger animations
  useEffect(() => {
    const prevAttachments = prevAttachmentsRef.current;

    // Check for newly added images (enter animation)
    if (attachments.length > prevAttachments.length) {
      const newIndices = new Set<number>();
      for (let i = prevAttachments.length; i < attachments.length; i++) {
        newIndices.add(i);
      }
      setEnteringIndices(newIndices);
      const timer = setTimeout(() => setEnteringIndices(new Set()), 250);
      prevAttachmentsRef.current = attachments;
      return () => clearTimeout(timer);
    }

    // Check for removed images (exit animation)
    if (attachments.length < prevAttachments.length && prevAttachments.length > 0) {
      // Find which images were removed
      const removedImages: ExitingImage[] = prevAttachments
        .filter((prev) => !attachments.some((curr) => curr.data === prev.data))
        .map((attachment, i) => ({
          attachment,
          key: `exiting-${Date.now()}-${i}`,
        }));

      if (removedImages.length > 0) {
        setExitingImages(removedImages);
        // Clear exiting images after animation completes
        const timer = setTimeout(() => setExitingImages([]), 150);
        prevAttachmentsRef.current = attachments;
        return () => clearTimeout(timer);
      }
    }

    prevAttachmentsRef.current = attachments;
  }, [attachments]);

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

  // Show if vision is supported OR if there are attachments/exiting images
  if (!supportsVision && !hasAttachments && !hasExitingImages) {
    return null;
  }

  const acceptFormats = capabilities?.supported_formats.join(",") ?? "image/*";

  return (
    <>
      {/* Inject keyframes for animations */}
      <style>{animationKeyframes}</style>

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

        {/* Image previews container */}
        {(hasAttachments || hasExitingImages) && (
          <div className="flex items-center gap-1.5 relative">
            {/* Exiting images (animating out) */}
            {exitingImages.map((exiting) => (
              <div
                key={exiting.key}
                className="relative"
                style={{
                  animation: "pop-out 0.15s ease-out forwards",
                }}
              >
                <img
                  src={exiting.attachment.data}
                  alt={exiting.attachment.filename || "Exiting image"}
                  className="h-8 w-8 rounded object-cover border border-[var(--border-medium)]"
                />
              </div>
            ))}

            {/* Active attachments */}
            {attachments.map((attachment, index) => (
              <div
                key={`${attachment.filename}-${index}`}
                className="relative group"
                style={
                  enteringIndices.has(index) ? { animation: "pop-in 0.15s ease-out" } : undefined
                }
              >
                <img
                  src={attachment.data}
                  alt={attachment.filename || `Image ${index + 1}`}
                  className="h-8 w-8 rounded object-cover border border-[var(--border-medium)]"
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
    </>
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
