import { Component } from "react";
import type { ErrorInfo, ReactNode } from "react";

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
		console.error("Uncaught error:", error, info);
	}

	render() {
		if (this.state.hasError) {
			return (
				<div className="flex min-h-screen items-center justify-center bg-gray-100">
					<div className="rounded-lg bg-white p-8 text-center shadow-lg">
						<h1 className="mb-4 text-2xl font-bold text-gray-900">
							Something went wrong
						</h1>
						<p className="mb-6 text-gray-600">
							An unexpected error occurred. Please try reloading the page.
						</p>
						<button
							type="button"
							onClick={() => window.location.reload()}
							className="rounded-md bg-blue-600 px-4 py-2 text-white hover:bg-blue-700"
						>
							Reload Page
						</button>
					</div>
				</div>
			);
		}

		return this.props.children;
	}
}
