import { Component } from "react";
import type { ErrorInfo, ReactNode } from "react";

interface Props {
  children: ReactNode;
}

interface State {
  error: Error | null;
}

export default class ErrorBoundary extends Component<Props, State> {
  state: State = { error: null };

  static getDerivedStateFromError(error: Error): State {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    // eslint-disable-next-line no-console
    console.error("D3R·AC UI crashed:", error, info.componentStack);
  }

  render() {
    if (this.state.error) {
      return (
        <div className="container" style={{ padding: "80px 24px", textAlign: "center" }}>
          <p className="eyebrow" style={{ marginBottom: 12 }}>Something went wrong</p>
          <h1 style={{ fontSize: 26, marginBottom: 12 }}>This page hit an error.</h1>
          <p style={{ color: "var(--text-muted)", marginBottom: 24 }}>
            Reloading usually fixes it. If it keeps happening, the console has the details.
          </p>
          <button className="btn btn-primary" onClick={() => window.location.reload()}>
            Reload
          </button>
        </div>
      );
    }
    return this.props.children;
  }
}
