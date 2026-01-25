import { AlertTriangle } from "lucide-react";
import { Component, type ErrorInfo, type ReactNode } from "react";

interface Props {
  /** Unique identifier for the block (for debugging) */
  blockId: string;
  /** Children to render */
  children: ReactNode;
}

interface State {
  hasError: boolean;
  error: Error | null;
}

/**
 * Error boundary for individual timeline blocks.
 *
 * Catches errors in child components and displays a fallback UI,
 * preventing one broken block from crashing the entire timeline.
 */
export class TimelineBlockErrorBoundary extends Component<Props, State> {
  constructor(props: Props) {
    super(props);
    this.state = { hasError: false, error: null };
  }

  static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo): void {
    // Log error for debugging
    console.error(`Timeline block error (${this.props.blockId}):`, error, errorInfo);
  }

  render() {
    if (this.state.hasError) {
      return (
        <div className="rounded-lg border border-[var(--ansi-red)] bg-[var(--ansi-red)]/10 p-3 space-y-2">
          <div className="flex items-center gap-2 text-sm text-[var(--ansi-red)]">
            <AlertTriangle className="w-4 h-4" />
            <span className="font-medium">Failed to render block</span>
          </div>
          <div className="text-xs text-muted-foreground">
            <p>Block ID: {this.props.blockId}</p>
            {this.state.error && (
              <p className="mt-1 font-mono text-[var(--ansi-red)]/80">{this.state.error.message}</p>
            )}
          </div>
        </div>
      );
    }

    return this.props.children;
  }
}
