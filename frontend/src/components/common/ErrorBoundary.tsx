import type { ErrorInfo, ReactNode } from 'react';
import { Component } from 'react';
import i18n from '@/lib/i18n';
import { logger } from '@/lib/logger';

interface Props {
  children: ReactNode;
}

interface State {
  hasError: boolean;
}

export class ErrorBoundary extends Component<Props, State> {
  state: State = { hasError: false };

  static getDerivedStateFromError(): State {
    return { hasError: true };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    logger.error({ err: error, componentStack: info.componentStack }, 'uncaught React error');
  }

  render() {
    if (this.state.hasError) {
      return (
        <div className="flex min-h-screen items-center justify-center bg-muted">
          <div className="rounded-lg bg-background p-8 text-center shadow-lg">
            <h1 className="mb-4 text-2xl font-bold text-foreground">
              {i18n.t('error.somethingWrong')}
            </h1>
            <p className="mb-6 text-muted-foreground">{i18n.t('error.unexpectedError')}</p>
            <button
              type="button"
              onClick={() => window.location.reload()}
              className="rounded-md bg-primary px-4 py-2 text-primary-foreground hover:bg-primary/90"
            >
              {i18n.t('common.reloadPage')}
            </button>
          </div>
        </div>
      );
    }

    return this.props.children;
  }
}
