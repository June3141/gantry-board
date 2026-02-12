import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { ErrorBoundary } from "./ErrorBoundary";

function ThrowingChild() {
	throw new Error("Test error");
}

function GoodChild() {
	return <div>All good</div>;
}

describe("ErrorBoundary", () => {
	it("renders children normally when no error", () => {
		render(
			<ErrorBoundary>
				<GoodChild />
			</ErrorBoundary>,
		);

		expect(screen.getByText("All good")).toBeInTheDocument();
	});

	it("shows fallback UI on error", () => {
		// Suppress console.error for expected error boundary logging
		vi.spyOn(console, "error").mockImplementation(() => {});

		render(
			<ErrorBoundary>
				<ThrowingChild />
			</ErrorBoundary>,
		);

		expect(
			screen.getByText("Something went wrong"),
		).toBeInTheDocument();
		expect(screen.getByRole("button", { name: /reload/i })).toBeInTheDocument();
	});

	it("reloads page when reload button is clicked", async () => {
		vi.spyOn(console, "error").mockImplementation(() => {});

		// Mock window.location.reload
		const reloadMock = vi.fn();
		Object.defineProperty(window, "location", {
			value: { ...window.location, reload: reloadMock },
			writable: true,
		});

		render(
			<ErrorBoundary>
				<ThrowingChild />
			</ErrorBoundary>,
		);

		const reloadButton = screen.getByRole("button", { name: /reload/i });
		await userEvent.click(reloadButton);

		expect(reloadMock).toHaveBeenCalled();
	});
});
