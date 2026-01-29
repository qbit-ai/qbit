import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { AgentMessage as AgentMessageType } from "@/store";
import { useStore } from "@/store";
import { AgentMessage } from "./AgentMessage";

// Mock the store
vi.mock("@/store", async () => {
  const actual = await vi.importActual("@/store");
  return {
    ...actual,
    useStore: vi.fn(() => undefined),
  };
});

// Helper to create test message
function createTestMessage(overrides: Partial<AgentMessageType> = {}): AgentMessageType {
  return {
    id: "test-message-id",
    sessionId: "test-session-id",
    role: "user",
    content: "Test message content",
    timestamp: new Date().toISOString(),
    ...overrides,
  };
}

// Attachment type from the store
type Attachment = { type: "image"; data: string; media_type?: string; filename?: string };

// Helper to create test attachment
function createTestAttachment(overrides: Partial<Attachment> = {}): Attachment {
  return {
    type: "image",
    data: "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==",
    media_type: "image/png",
    filename: "test-image.png",
    ...overrides,
  };
}

describe("AgentMessage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useStore).mockReturnValue(undefined);
  });

  describe("user message rendering", () => {
    it("should render user message content", () => {
      const message = createTestMessage({
        role: "user",
        content: "Hello, this is my message",
      });

      render(<AgentMessage message={message} />);

      expect(screen.getByText("Hello, this is my message")).toBeInTheDocument();
    });

    it("should render image attachments in user message", () => {
      const message = createTestMessage({
        role: "user",
        content: "Check out this image",
        attachments: [
          createTestAttachment({ filename: "screenshot.png" }),
          createTestAttachment({ filename: "photo.jpg" }),
        ],
      });

      render(<AgentMessage message={message} />);

      expect(screen.getByAltText("screenshot.png")).toBeInTheDocument();
      expect(screen.getByAltText("photo.jpg")).toBeInTheDocument();
    });

    it("should render user message with only image (no text)", () => {
      const message = createTestMessage({
        role: "user",
        content: "",
        attachments: [createTestAttachment()],
      });

      render(<AgentMessage message={message} />);

      expect(screen.getByAltText("test-image.png")).toBeInTheDocument();
    });

    it("should use fallback alt text when filename is not provided", () => {
      const message = createTestMessage({
        role: "user",
        attachments: [createTestAttachment({ filename: undefined })],
      });

      render(<AgentMessage message={message} />);

      expect(screen.getByAltText("Attachment 1")).toBeInTheDocument();
    });

    it("should render images as clickable buttons", () => {
      const message = createTestMessage({
        role: "user",
        attachments: [createTestAttachment()],
      });

      render(<AgentMessage message={message} />);

      const imageButton = screen.getByRole("button", { name: /test-image\.png/i });
      expect(imageButton).toBeInTheDocument();
    });
  });

  describe("image modal interaction", () => {
    it("should open modal when image is clicked", async () => {
      const user = userEvent.setup();
      const message = createTestMessage({
        role: "user",
        attachments: [createTestAttachment({ filename: "clickable.png" })],
      });

      render(<AgentMessage message={message} />);

      // Click on the image thumbnail
      const imageButton = screen.getByRole("button", { name: /clickable\.png/i });
      await user.click(imageButton);

      // Modal should appear
      expect(screen.getByRole("dialog")).toBeInTheDocument();
    });

    it("should show expanded image in modal", async () => {
      const user = userEvent.setup();
      const testData = "data:image/png;base64,testdata123";
      const message = createTestMessage({
        role: "user",
        attachments: [createTestAttachment({ data: testData, filename: "expand-me.png" })],
      });

      render(<AgentMessage message={message} />);

      // Click on the image
      await user.click(screen.getByRole("button", { name: /expand-me\.png/i }));

      // Check the modal image has the correct src
      const dialog = screen.getByRole("dialog");
      const modalImage = within(dialog).getByRole("img");
      expect(modalImage).toHaveAttribute("src", testData);
    });

    it("should close modal when close button is clicked", async () => {
      const user = userEvent.setup();
      const message = createTestMessage({
        role: "user",
        attachments: [createTestAttachment()],
      });

      render(<AgentMessage message={message} />);

      // Open modal
      await user.click(screen.getByRole("button", { name: /test-image\.png/i }));
      expect(screen.getByRole("dialog")).toBeInTheDocument();

      // Close modal
      await user.click(screen.getByLabelText("Close image"));

      // Modal should be gone
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    });

    it("should close modal when Escape is pressed", async () => {
      const user = userEvent.setup();
      const message = createTestMessage({
        role: "user",
        attachments: [createTestAttachment()],
      });

      render(<AgentMessage message={message} />);

      // Open modal
      await user.click(screen.getByRole("button", { name: /test-image\.png/i }));
      expect(screen.getByRole("dialog")).toBeInTheDocument();

      // Press Escape
      await user.keyboard("{Escape}");

      // Modal should be gone
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    });
  });

  describe("assistant message rendering", () => {
    it("should render assistant message without attachments section", () => {
      const message = createTestMessage({
        role: "assistant",
        content: "I am an assistant response",
      });

      render(<AgentMessage message={message} />);

      expect(screen.getByText("I am an assistant response")).toBeInTheDocument();
    });
  });

  describe("copy button", () => {
    it("should show copy button for user messages with content", () => {
      const message = createTestMessage({
        role: "user",
        content: "Copy this text",
      });

      render(<AgentMessage message={message} />);

      expect(screen.getByTestId("user-message-copy-button")).toBeInTheDocument();
    });
  });
});
