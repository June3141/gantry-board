import { afterEach, describe, expect, it } from 'vitest';
import { useAgentStore } from './agentStore';

describe('agentStore', () => {
  afterEach(() => {
    useAgentStore.getState().reset();
  });

  it('has correct initial state', () => {
    const state = useAgentStore.getState();
    expect(state.activeSessionId).toBeNull();
    expect(state.outputLines).toEqual([]);
    expect(state.isStarting).toBe(false);
  });

  it('sets active session', () => {
    useAgentStore.getState().setActiveSession('session-1');
    expect(useAgentStore.getState().activeSessionId).toBe('session-1');
  });

  it('clears active session', () => {
    useAgentStore.getState().setActiveSession('session-1');
    useAgentStore.getState().setActiveSession(null);
    expect(useAgentStore.getState().activeSessionId).toBeNull();
  });

  it('appends output line', () => {
    useAgentStore.getState().appendOutput('Hello');
    useAgentStore.getState().appendOutput('World');
    expect(useAgentStore.getState().outputLines).toEqual(['Hello', 'World']);
  });

  it('clears output on reset', () => {
    useAgentStore.getState().appendOutput('test');
    useAgentStore.getState().setActiveSession('session-1');
    useAgentStore.getState().reset();
    expect(useAgentStore.getState().activeSessionId).toBeNull();
    expect(useAgentStore.getState().outputLines).toEqual([]);
  });

  it('sets isStarting flag', () => {
    useAgentStore.getState().setStarting(true);
    expect(useAgentStore.getState().isStarting).toBe(true);
    useAgentStore.getState().setStarting(false);
    expect(useAgentStore.getState().isStarting).toBe(false);
  });

  it('sets output lines (bulk load for history)', () => {
    useAgentStore.getState().setOutputLines(['line 1', 'line 2', 'line 3']);
    expect(useAgentStore.getState().outputLines).toEqual(['line 1', 'line 2', 'line 3']);
  });

  it('replaces existing output when setOutputLines is called', () => {
    useAgentStore.getState().appendOutput('old line');
    useAgentStore.getState().setOutputLines(['new line 1', 'new line 2']);
    expect(useAgentStore.getState().outputLines).toEqual(['new line 1', 'new line 2']);
  });

  it('tracks isLoadingHistory state', () => {
    expect(useAgentStore.getState().isLoadingHistory).toBe(false);
    useAgentStore.getState().setLoadingHistory(true);
    expect(useAgentStore.getState().isLoadingHistory).toBe(true);
    useAgentStore.getState().setLoadingHistory(false);
    expect(useAgentStore.getState().isLoadingHistory).toBe(false);
  });

  it('resets isLoadingHistory on reset', () => {
    useAgentStore.getState().setLoadingHistory(true);
    useAgentStore.getState().reset();
    expect(useAgentStore.getState().isLoadingHistory).toBe(false);
  });

  it('limits output to 1000 lines', () => {
    for (let i = 0; i < 1005; i++) {
      useAgentStore.getState().appendOutput(`line ${i}`);
    }
    const lines = useAgentStore.getState().outputLines;
    expect(lines.length).toBe(1000);
    expect(lines[lines.length - 1]).toBe('line 1004');
  });
});
