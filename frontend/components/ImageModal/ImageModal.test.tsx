import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { ImageModal } from "./index";

describe("ImageModal", () => {
  const mockOnClose = vi.fn();
  const testImageSrc =
    "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==";

  beforeEach(() => {
    vi.clearAllMocks();
    // Reset body overflow style
    document.body.style.overflow = "";
  });

  describe("rendering", () => {
    it("should not render when open is false", () => {
      const { container } = render(
        <ImageModal src={testImageSrc} open={false} onClose={mockOnClose} />
      );

      expect(container.firstChild).toBeNull();
    });

    it("should render when open is true", () => {
      render(<ImageModal src={testImageSrc} open={true} onClose={mockOnClose} />);

      expect(screen.getByRole("dialog")).toBeInTheDocument();
    });

    it("should render the image with correct src", () => {
      render(<ImageModal src={testImageSrc} open={true} onClose={mockOnClose} />);

      const img = screen.getByRole("img");
      expect(img).toHaveAttribute("src", testImageSrc);
    });

    it("should use provided alt text", () => {
      render(
        <ImageModal src={testImageSrc} alt="Test screenshot" open={true} onClose={mockOnClose} />
      );

      expect(screen.getByAltText("Test screenshot")).toBeInTheDocument();
    });

    it("should use default alt text when not provided", () => {
      render(<ImageModal src={testImageSrc} open={true} onClose={mockOnClose} />);

      expect(screen.getByAltText("Expanded image")).toBeInTheDocument();
    });

    it("should render close button", () => {
      render(<ImageModal src={testImageSrc} open={true} onClose={mockOnClose} />);

      expect(screen.getByLabelText("Close image")).toBeInTheDocument();
    });

    it("should have correct aria attributes for accessibility", () => {
      render(<ImageModal src={testImageSrc} open={true} onClose={mockOnClose} />);

      const dialog = screen.getByRole("dialog");
      expect(dialog).toHaveAttribute("aria-modal", "true");
      expect(dialog).toHaveAttribute("aria-label", "Expanded image view");
    });
  });

  describe("closing behavior", () => {
    it("should call onClose when close button is clicked", async () => {
      const user = userEvent.setup();
      render(<ImageModal src={testImageSrc} open={true} onClose={mockOnClose} />);

      await user.click(screen.getByLabelText("Close image"));

      expect(mockOnClose).toHaveBeenCalledTimes(1);
    });

    it("should call onClose when backdrop is clicked", async () => {
      const user = userEvent.setup();
      render(<ImageModal src={testImageSrc} open={true} onClose={mockOnClose} />);

      // Click on the backdrop (dialog element itself)
      const dialog = screen.getByRole("dialog");
      await user.click(dialog);

      expect(mockOnClose).toHaveBeenCalledTimes(1);
    });

    it("should NOT call onClose when image is clicked", async () => {
      const user = userEvent.setup();
      render(<ImageModal src={testImageSrc} open={true} onClose={mockOnClose} />);

      // Click on the image itself
      const img = screen.getByRole("img");
      await user.click(img);

      expect(mockOnClose).not.toHaveBeenCalled();
    });

    it("should call onClose when Escape key is pressed", async () => {
      const user = userEvent.setup();
      render(<ImageModal src={testImageSrc} open={true} onClose={mockOnClose} />);

      await user.keyboard("{Escape}");

      expect(mockOnClose).toHaveBeenCalledTimes(1);
    });
  });

  describe("body scroll lock", () => {
    it("should prevent body scroll when open", () => {
      render(<ImageModal src={testImageSrc} open={true} onClose={mockOnClose} />);

      expect(document.body.style.overflow).toBe("hidden");
    });

    it("should restore body scroll when closed", () => {
      const { rerender } = render(
        <ImageModal src={testImageSrc} open={true} onClose={mockOnClose} />
      );

      expect(document.body.style.overflow).toBe("hidden");

      rerender(<ImageModal src={testImageSrc} open={false} onClose={mockOnClose} />);

      expect(document.body.style.overflow).toBe("");
    });

    it("should restore body scroll on unmount", () => {
      const { unmount } = render(
        <ImageModal src={testImageSrc} open={true} onClose={mockOnClose} />
      );

      expect(document.body.style.overflow).toBe("hidden");

      unmount();

      expect(document.body.style.overflow).toBe("");
    });
  });
});
