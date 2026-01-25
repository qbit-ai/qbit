import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { TimelineBlockErrorBoundary } from "./TimelineBlockErrorBoundary";

// Component that throws an error
function ThrowingComponent({ shouldThrow }: { shouldThrow: boolean }) {
  if (shouldThrow) {
    throw new Error("Test error from component");
  }
  return <div>Normal content</div>;
}

describe("TimelineBlockErrorBoundary", () => {
  // Suppress console.error for expected errors in tests
  beforeEach(() => {
    vi.spyOn(console, "error").mockImplementation(() => {});
  });

  describe("normal rendering", () => {
    it("should render children when no error", () => {
      render(
        <TimelineBlockErrorBoundary blockId="block-1">
          <div>Child content</div>
        </TimelineBlockErrorBoundary>
      );

      expect(screen.getByText("Child content")).toBeInTheDocument();
    });

    it("should pass through multiple children", () => {
      render(
        <TimelineBlockErrorBoundary blockId="block-1">
          <div>First</div>
          <div>Second</div>
        </TimelineBlockErrorBoundary>
      );

      expect(screen.getByText("First")).toBeInTheDocument();
      expect(screen.getByText("Second")).toBeInTheDocument();
    });
  });

  describe("error handling", () => {
    it("should catch errors and render fallback UI", () => {
      render(
        <TimelineBlockErrorBoundary blockId="block-1">
          <ThrowingComponent shouldThrow={true} />
        </TimelineBlockErrorBoundary>
      );

      expect(screen.getByText(/Failed to render/)).toBeInTheDocument();
      expect(screen.queryByText("Normal content")).not.toBeInTheDocument();
    });

    it("should display block ID in error message", () => {
      render(
        <TimelineBlockErrorBoundary blockId="my-block-id">
          <ThrowingComponent shouldThrow={true} />
        </TimelineBlockErrorBoundary>
      );

      expect(screen.getByText(/my-block-id/)).toBeInTheDocument();
    });

    it("should display error message when available", () => {
      render(
        <TimelineBlockErrorBoundary blockId="block-1">
          <ThrowingComponent shouldThrow={true} />
        </TimelineBlockErrorBoundary>
      );

      expect(screen.getByText(/Test error from component/)).toBeInTheDocument();
    });
  });

  describe("recovery", () => {
    it("should allow re-render after error is fixed", () => {
      const { rerender } = render(
        <TimelineBlockErrorBoundary blockId="block-1">
          <ThrowingComponent shouldThrow={true} />
        </TimelineBlockErrorBoundary>
      );

      // Error state
      expect(screen.getByText(/Failed to render/)).toBeInTheDocument();

      // Re-render with non-throwing component
      // Note: Error boundaries need a key change to reset
      rerender(
        <TimelineBlockErrorBoundary blockId="block-1" key="new-key">
          <ThrowingComponent shouldThrow={false} />
        </TimelineBlockErrorBoundary>
      );

      expect(screen.getByText("Normal content")).toBeInTheDocument();
    });
  });

  describe("styling", () => {
    it("should render with error styling (red border)", () => {
      render(
        <TimelineBlockErrorBoundary blockId="block-1">
          <ThrowingComponent shouldThrow={true} />
        </TimelineBlockErrorBoundary>
      );

      const errorContainer = document.querySelector(".border-\\[var\\(--ansi-red\\)\\]");
      expect(errorContainer).toBeInTheDocument();
    });
  });
});
