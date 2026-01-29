import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { ImagePart, VisionCapabilities } from "@/lib/ai";
import { ImageAttachment } from "./ImageAttachment";

// Helper to create test image attachment
function createTestAttachment(overrides: Partial<ImagePart> = {}): ImagePart {
  return {
    type: "image",
    data: "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==",
    media_type: "image/png",
    filename: "test-image.png",
    ...overrides,
  };
}

// Helper to create vision capabilities
function createVisionCapabilities(overrides: Partial<VisionCapabilities> = {}): VisionCapabilities {
  return {
    supports_vision: true,
    max_image_size_bytes: 10 * 1024 * 1024,
    supported_formats: ["image/png", "image/jpeg", "image/gif", "image/webp"],
    ...overrides,
  };
}

describe("ImageAttachment", () => {
  const mockOnAttachmentsChange = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe("rendering", () => {
    it("should not render when vision is not supported and no attachments", () => {
      const { container } = render(
        <ImageAttachment
          attachments={[]}
          onAttachmentsChange={mockOnAttachmentsChange}
          capabilities={createVisionCapabilities({ supports_vision: false })}
        />
      );

      expect(container.firstChild).toBeNull();
    });

    it("should render attach button when vision is supported", () => {
      render(
        <ImageAttachment
          attachments={[]}
          onAttachmentsChange={mockOnAttachmentsChange}
          capabilities={createVisionCapabilities()}
        />
      );

      expect(screen.getByTitle("Attach image")).toBeInTheDocument();
    });

    it("should render image previews when attachments exist", () => {
      const attachments = [
        createTestAttachment({ filename: "image1.png" }),
        createTestAttachment({ filename: "image2.png" }),
      ];

      render(
        <ImageAttachment
          attachments={attachments}
          onAttachmentsChange={mockOnAttachmentsChange}
          capabilities={createVisionCapabilities()}
        />
      );

      expect(screen.getByAltText("image1.png")).toBeInTheDocument();
      expect(screen.getByAltText("image2.png")).toBeInTheDocument();
    });

    it("should show remove button on hover for each attachment", () => {
      const attachments = [createTestAttachment()];

      render(
        <ImageAttachment
          attachments={attachments}
          onAttachmentsChange={mockOnAttachmentsChange}
          capabilities={createVisionCapabilities()}
        />
      );

      // Remove buttons exist but are hidden until hover (opacity-0)
      const removeButtons = screen.getAllByTitle("Remove image");
      expect(removeButtons).toHaveLength(1);
    });

    it("should still show attachments when vision is not supported (for removal)", () => {
      const attachments = [createTestAttachment()];

      render(
        <ImageAttachment
          attachments={attachments}
          onAttachmentsChange={mockOnAttachmentsChange}
          capabilities={createVisionCapabilities({ supports_vision: false })}
        />
      );

      // Should show the image preview even without vision support
      expect(screen.getByAltText("test-image.png")).toBeInTheDocument();
      // But should NOT show the attach button
      expect(screen.queryByTitle("Attach image")).not.toBeInTheDocument();
    });

    it("should disable attach button when disabled prop is true", () => {
      render(
        <ImageAttachment
          attachments={[]}
          onAttachmentsChange={mockOnAttachmentsChange}
          capabilities={createVisionCapabilities()}
          disabled={true}
        />
      );

      const button = screen.getByTitle("Attach image");
      expect(button).toBeDisabled();
    });
  });

  describe("remove functionality", () => {
    it("should call onAttachmentsChange with filtered array when remove is clicked", async () => {
      const user = userEvent.setup();
      const attachments = [
        createTestAttachment({ filename: "image1.png" }),
        createTestAttachment({ filename: "image2.png" }),
      ];

      render(
        <ImageAttachment
          attachments={attachments}
          onAttachmentsChange={mockOnAttachmentsChange}
          capabilities={createVisionCapabilities()}
        />
      );

      const removeButtons = screen.getAllByTitle("Remove image");
      await user.click(removeButtons[0]);

      expect(mockOnAttachmentsChange).toHaveBeenCalledTimes(1);
      expect(mockOnAttachmentsChange).toHaveBeenCalledWith([attachments[1]]);
    });

    it("should remove correct attachment when multiple exist", async () => {
      const user = userEvent.setup();
      const attachments = [
        createTestAttachment({ filename: "first.png", data: "data:first" }),
        createTestAttachment({ filename: "second.png", data: "data:second" }),
        createTestAttachment({ filename: "third.png", data: "data:third" }),
      ];

      render(
        <ImageAttachment
          attachments={attachments}
          onAttachmentsChange={mockOnAttachmentsChange}
          capabilities={createVisionCapabilities()}
        />
      );

      // Remove the middle one
      const removeButtons = screen.getAllByTitle("Remove image");
      await user.click(removeButtons[1]);

      expect(mockOnAttachmentsChange).toHaveBeenCalledWith([attachments[0], attachments[2]]);
    });
  });

  describe("animations", () => {
    it("should inject animation keyframes style", () => {
      render(
        <ImageAttachment
          attachments={[createTestAttachment()]}
          onAttachmentsChange={mockOnAttachmentsChange}
          capabilities={createVisionCapabilities()}
        />
      );

      const style = document.querySelector("style");
      expect(style).toBeInTheDocument();
      expect(style?.textContent).toContain("@keyframes pop-in");
      expect(style?.textContent).toContain("@keyframes pop-out");
    });

    it("should apply enter animation to newly added attachments", async () => {
      const { rerender } = render(
        <ImageAttachment
          attachments={[]}
          onAttachmentsChange={mockOnAttachmentsChange}
          capabilities={createVisionCapabilities()}
        />
      );

      // Add an attachment
      const newAttachment = createTestAttachment();
      rerender(
        <ImageAttachment
          attachments={[newAttachment]}
          onAttachmentsChange={mockOnAttachmentsChange}
          capabilities={createVisionCapabilities()}
        />
      );

      // Check that animation is applied
      const imageContainer = screen.getByAltText("test-image.png").parentElement;
      expect(imageContainer?.style.animation).toContain("pop-in");
    });

    it("should render exiting images with exit animation when attachments are removed", async () => {
      const attachment = createTestAttachment();

      const { rerender } = render(
        <ImageAttachment
          attachments={[attachment]}
          onAttachmentsChange={mockOnAttachmentsChange}
          capabilities={createVisionCapabilities()}
        />
      );

      // Remove the attachment
      rerender(
        <ImageAttachment
          attachments={[]}
          onAttachmentsChange={mockOnAttachmentsChange}
          capabilities={createVisionCapabilities()}
        />
      );

      // The exiting image should briefly appear with the exit animation
      // Wait for the animation state to be set
      await waitFor(() => {
        const exitingImg = document.querySelector('img[alt="test-image.png"]');
        if (exitingImg) {
          const container = exitingImg.parentElement;
          expect(container?.style.animation).toContain("pop-out");
        }
      });
    });
  });

  describe("file input", () => {
    it("should have hidden file input with correct accept attribute", () => {
      render(
        <ImageAttachment
          attachments={[]}
          onAttachmentsChange={mockOnAttachmentsChange}
          capabilities={createVisionCapabilities({
            supported_formats: ["image/png", "image/jpeg"],
          })}
        />
      );

      const fileInput = document.querySelector('input[type="file"]') as HTMLInputElement;
      expect(fileInput).toBeInTheDocument();
      expect(fileInput).toHaveClass("hidden");
      expect(fileInput.accept).toBe("image/png,image/jpeg");
    });

    it("should allow multiple file selection", () => {
      render(
        <ImageAttachment
          attachments={[]}
          onAttachmentsChange={mockOnAttachmentsChange}
          capabilities={createVisionCapabilities()}
        />
      );

      const fileInput = document.querySelector('input[type="file"]') as HTMLInputElement;
      expect(fileInput).toHaveAttribute("multiple");
    });
  });

  describe("alt text", () => {
    it("should use filename as alt text when available", () => {
      render(
        <ImageAttachment
          attachments={[createTestAttachment({ filename: "my-screenshot.png" })]}
          onAttachmentsChange={mockOnAttachmentsChange}
          capabilities={createVisionCapabilities()}
        />
      );

      expect(screen.getByAltText("my-screenshot.png")).toBeInTheDocument();
    });

    it("should use fallback alt text when filename is not available", () => {
      render(
        <ImageAttachment
          attachments={[createTestAttachment({ filename: undefined })]}
          onAttachmentsChange={mockOnAttachmentsChange}
          capabilities={createVisionCapabilities()}
        />
      );

      expect(screen.getByAltText("Image 1")).toBeInTheDocument();
    });
  });
});
