import { describe, expect, it } from 'vitest';
import { extractErrorMessage } from './errorHandler';

describe('extractErrorMessage', () => {
  it('extracts message from standard API error response', () => {
    const error = new Error('{"error":{"message":"Not found","code":"NOT_FOUND"}}');
    expect(extractErrorMessage(error)).toBe('Not found');
  });

  it('falls back to Error.message when not JSON', () => {
    const error = new Error('Network error');
    expect(extractErrorMessage(error)).toBe('Network error');
  });

  it('falls back to default message for unknown errors', () => {
    expect(extractErrorMessage('something')).toBe('An unexpected error occurred.');
  });

  it('uses custom fallback when provided', () => {
    expect(extractErrorMessage(null, 'Custom fallback')).toBe('Custom fallback');
  });

  it('handles nested error.error.message structure', () => {
    const error = new Error(
      JSON.stringify({
        error: {
          message: 'Validation failed',
          code: 'VALIDATION_FAILED',
          details: ['field required'],
        },
      }),
    );
    expect(extractErrorMessage(error)).toBe('Validation failed');
  });

  it('handles error.error as string (legacy)', () => {
    const error = new Error(JSON.stringify({ error: 'Bad request' }));
    expect(extractErrorMessage(error)).toBe('Bad request');
  });
});
