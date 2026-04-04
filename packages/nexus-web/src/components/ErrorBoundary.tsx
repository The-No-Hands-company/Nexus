import { Component, type ErrorInfo, type ReactNode } from "react";

interface Props {
  children: ReactNode;
  /** Custom fallback UI. Receives the error for display. */
  fallback?: (error: Error, reset: () => void) => ReactNode;
}

interface State {
  error: Error | null;
}

/**
 * ErrorBoundary — catches unhandled rendering errors anywhere in the subtree
 * and shows a recovery UI instead of a blank screen.
 *
 * Usage:
 *   <ErrorBoundary>
 *     <MyComponent />
 *   </ErrorBoundary>
 */
export class ErrorBoundary extends Component<Props, State> {
  constructor(props: Props) {
    super(props);
    this.state = { error: null };
  }

  static getDerivedStateFromError(error: Error): State {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    // Log to console in development; in production this is where you'd
    // send to a Sentry-compatible error reporting endpoint.
    console.error("[Nexus] Unhandled rendering error:", error, info.componentStack);
  }

  reset = () => this.setState({ error: null });

  render() {
    const { error } = this.state;
    if (error) {
      if (this.props.fallback) {
        return this.props.fallback(error, this.reset);
      }
      return <DefaultErrorFallback error={error} onReset={this.reset} />;
    }
    return this.props.children;
  }
}

// ── Default fallback UI ───────────────────────────────────────────────────────

function DefaultErrorFallback({
  error,
  onReset,
}: {
  error: Error;
  onReset: () => void;
}) {
  return (
    <div
      role="alert"
      className="flex flex-col items-center justify-center h-full p-8 text-center bg-bg-900"
    >
      <div className="text-4xl mb-4">⚠️</div>
      <h2 className="text-lg font-semibold text-fg mb-2">
        Something went wrong
      </h2>
      <p className="text-sm text-muted mb-6 max-w-sm">
        An unexpected error occurred. You can try to recover, or reload the page
        if the problem persists.
      </p>
      <details className="text-xs text-muted/60 bg-bg-700 rounded px-3 py-2 mb-6 max-w-md text-left">
        <summary className="cursor-pointer hover:text-muted">
          Error details
        </summary>
        <pre className="mt-2 whitespace-pre-wrap break-all">{error.message}</pre>
      </details>
      <div className="flex gap-3">
        <button
          onClick={onReset}
          className="px-4 py-2 bg-accent-500 hover:bg-accent-400 text-white rounded text-sm font-medium transition-colors"
        >
          Try to recover
        </button>
        <button
          onClick={() => window.location.reload()}
          className="px-4 py-2 bg-bg-700 hover:bg-bg-600 text-muted rounded text-sm font-medium transition-colors"
        >
          Reload page
        </button>
      </div>
    </div>
  );
}
