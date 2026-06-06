import { Component, type ErrorInfo, type ReactNode } from "react";

interface ErrorBoundaryFallbackArgs {
  error: Error;
  reset: () => void;
}

interface ErrorBoundaryProps {
  children: ReactNode;
  fallback: (args: ErrorBoundaryFallbackArgs) => ReactNode;
  onError?: (error: Error, info: ErrorInfo) => void;
  resetKey?: string;
}

interface ErrorBoundaryState {
  error: Error | null;
}

export class ErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  state: ErrorBoundaryState = {
    error: null
  };

  static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error("[fc-web] render failure", error, info);
    this.props.onError?.(error, info);
  }

  componentDidUpdate(prevProps: ErrorBoundaryProps) {
    if (this.state.error && this.props.resetKey !== prevProps.resetKey) {
      this.setState({ error: null });
    }
  }

  private reset = () => {
    this.setState({ error: null });
  };

  render() {
    if (this.state.error) {
      return this.props.fallback({
        error: this.state.error,
        reset: this.reset
      });
    }

    return this.props.children;
  }
}
