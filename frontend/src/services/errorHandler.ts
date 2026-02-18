const DEFAULT_MESSAGE = 'An unexpected error occurred.';

/**
 * Extract a user-facing error message from an API error.
 *
 * Handles both the standardized `{ error: { message, code } }` format
 * and the legacy `{ error: "string" }` format.
 */
export function extractErrorMessage(error: unknown, fallback: string = DEFAULT_MESSAGE): string {
  if (error instanceof Error) {
    try {
      const body = JSON.parse(error.message);
      if (typeof body?.error === 'object' && body.error.message) {
        return body.error.message;
      }
      if (typeof body?.error === 'string') {
        return body.error;
      }
    } catch {
      // Not JSON — use the raw message
    }
    return error.message || fallback;
  }
  return fallback;
}
