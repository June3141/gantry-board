import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { logger } from './logger';

describe('logger', () => {
  let consoleSpy: ReturnType<typeof vi.spyOn>;

  beforeEach(() => {
    // pino browser mode writes to console
    consoleSpy = vi.spyOn(console, 'log').mockImplementation(() => {});
    vi.spyOn(console, 'error').mockImplementation(() => {});
    vi.spyOn(console, 'warn').mockImplementation(() => {});
    vi.spyOn(console, 'info').mockImplementation(() => {});
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('exports a logger object with standard log methods', () => {
    expect(typeof logger.info).toBe('function');
    expect(typeof logger.warn).toBe('function');
    expect(typeof logger.error).toBe('function');
    expect(typeof logger.debug).toBe('function');
  });

  it('supports child loggers with context', () => {
    const child = logger.child({ module: 'sse' });
    expect(typeof child.info).toBe('function');
    expect(typeof child.error).toBe('function');
  });

  it('can log structured data with message', () => {
    // Should not throw
    expect(() => logger.info({ event: 'test' }, 'test message')).not.toThrow();
  });

  it('can log errors with context', () => {
    const err = new Error('test error');
    expect(() => logger.error({ err, url: '/api/test' }, 'request failed')).not.toThrow();
  });
});
