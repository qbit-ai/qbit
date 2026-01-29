import { X } from "lucide-react";
import { useCallback, useEffect } from "react";
import { cn } from "@/lib/utils";

interface ImageModalProps {
  src: string;
  alt?: string;
  open: boolean;
  onClose: () => void;
}

export function ImageModal({ src, alt, open, onClose }: ImageModalProps) {
  // Handle escape key to close
  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        onClose();
      }
    },
    [onClose]
  );

  useEffect(() => {
    if (open) {
      document.addEventListener("keydown", handleKeyDown);
      // Prevent body scroll when modal is open
      document.body.style.overflow = "hidden";
    }
    return () => {
      document.removeEventListener("keydown", handleKeyDown);
      document.body.style.overflow = "";
    };
  }, [open, handleKeyDown]);

  if (!open) return null;

  return (
    // biome-ignore lint/a11y/useKeyWithClickEvents: Escape key handled via global listener
    <div
      role="dialog"
      aria-modal="true"
      aria-label="Expanded image view"
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/80 backdrop-blur-sm"
      onClick={onClose}
    >
      {/* Close button */}
      <button
        type="button"
        onClick={(e) => {
          e.stopPropagation();
          onClose();
        }}
        className={cn(
          "absolute top-4 right-4 z-10",
          "h-8 w-8 flex items-center justify-center rounded-full",
          "bg-background/80 hover:bg-background text-foreground",
          "transition-colors"
        )}
        aria-label="Close image"
      >
        <X className="w-4 h-4" />
      </button>

      {/* Image container - click on image doesn't close */}
      {/* biome-ignore lint/a11y/useKeyWithClickEvents: stopPropagation only, no action */}
      {/* biome-ignore lint/a11y/noStaticElementInteractions: prevents close when clicking image */}
      <div
        className="relative max-w-[90vw] max-h-[90vh] flex items-center justify-center"
        onClick={(e) => e.stopPropagation()}
      >
        <img
          src={src}
          alt={alt || "Expanded image"}
          className="max-w-full max-h-[90vh] object-contain rounded-lg shadow-2xl"
        />
      </div>
    </div>
  );
}
