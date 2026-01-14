import { Component, type ReactNode } from "react";
import { logger } from "@/lib/logger";

interface ErrorBoundaryProps {
  children: ReactNode;
  /** Optional fallback to render when an error occurs. If not provided, children continue to render after error is logged. */
  fallback?: ReactNode;
  /** Called when an error is caught */
  onError?: (error: Error, errorInfo: React.ErrorInfo) => void;
}

interface ErrorBoundaryState {
  hasError: boolean;
  error: Error | null;
}

/**
 * Error boundary component that catches errors in its child component tree.
 *
 * Unlike typical error boundaries that show a fallback UI, this one logs the error
 * and continues rendering children by default. This allows the app to keep working
 * even when individual components throw errors.
 *
 * To show a fallback UI instead, pass the `fallback` prop.
 */
export class ErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  constructor(props: ErrorBoundaryProps) {
    super(props);
    this.state = { hasError: false, error: null };
  }

  static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, errorInfo: React.ErrorInfo) {
    // Log the error to our logging system
    logger.error("[ErrorBoundary] Caught error:", error.message, {
      componentStack: errorInfo.componentStack,
      stack: error.stack,
    });

    // Call optional error callback
    this.props.onError?.(error, errorInfo);

    // Reset state after a short delay to allow children to continue rendering
    // This enables "recovery" behavior where transient errors don't break the UI
    if (!this.props.fallback) {
      setTimeout(() => {
        this.setState({ hasError: false, error: null });
      }, 100);
    }
  }

  render() {
    // If a fallback is provided and we have an error, show the fallback
    if (this.state.hasError && this.props.fallback) {
      return this.props.fallback;
    }

    // Otherwise, continue rendering children (error is logged but UI continues)
    return this.props.children;
  }
}

/**
 * Sets up global error handlers for uncaught errors and unhandled promise rejections.
 * Call this once at app startup (in main.tsx).
 *
 * These handlers log errors but don't interrupt the app, allowing it to continue
 * functioning even when errors occur.
 */
export function setupGlobalErrorHandlers(): void {
  // Handle uncaught errors
  window.onerror = (message, source, lineno, colno, error) => {
    logger.error("[GlobalError] Uncaught error:", {
      message: String(message),
      source,
      lineno,
      colno,
      error: error?.message,
      stack: error?.stack,
    });

    // Return true to prevent the browser's default error handling (which would show an error overlay)
    // This allows the app to continue running
    return true;
  };

  // Handle unhandled promise rejections
  window.onunhandledrejection = (event) => {
    logger.error("[GlobalError] Unhandled promise rejection:", {
      reason: event.reason instanceof Error ? event.reason.message : String(event.reason),
      stack: event.reason instanceof Error ? event.reason.stack : undefined,
    });

    // Prevent the browser from logging the rejection to console (we already logged it)
    event.preventDefault();
  };
}
