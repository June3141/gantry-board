import { describe, expect, it } from 'vitest';
import type { AgentSession, TaskComment } from '../api/generated/model';
import { AgentSessionStatus, AgentType } from '../api/generated/model';
import { type TimelineItem, mergeTimeline } from './TaskTimeline';

const createComment = (overrides: Partial<TaskComment> = {}): TaskComment => ({
  id: 'comment-1',
  task_id: 'task-1',
  user_id: 'user-1',
  user_name: 'Alice',
  content: 'Test comment',
  created_at: '2026-01-01T12:00:00Z',
  updated_at: '2026-01-01T12:00:00Z',
  ...overrides,
});

const createSession = (overrides: Partial<AgentSession> = {}): AgentSession => ({
  id: 'session-1',
  task_id: 'task-1',
  agent_type: AgentType.claude_code,
  status: AgentSessionStatus.completed,
  created_at: '2026-01-01T10:00:00Z',
  updated_at: '2026-01-01T10:00:00Z',
  ...overrides,
});

describe('mergeTimeline', () => {
  it('merges comments and sessions sorted by created_at descending', () => {
    const comments = [
      createComment({ id: 'c1', created_at: '2026-01-01T12:00:00Z' }),
      createComment({ id: 'c2', created_at: '2026-01-01T08:00:00Z' }),
    ];
    const sessions = [createSession({ id: 's1', created_at: '2026-01-01T10:00:00Z' })];

    const result = mergeTimeline(comments, sessions);

    expect(result).toHaveLength(3);
    expect(result[0]).toEqual({ type: 'comment', data: comments[0] });
    expect(result[1]).toEqual({ type: 'agent_session', data: sessions[0] });
    expect(result[2]).toEqual({ type: 'comment', data: comments[1] });
  });

  it('returns only sessions when comments are empty', () => {
    const sessions = [createSession({ id: 's1' })];
    const result = mergeTimeline([], sessions);

    expect(result).toHaveLength(1);
    expect(result[0].type).toBe('agent_session');
  });

  it('returns only comments when sessions are empty', () => {
    const comments = [createComment({ id: 'c1' })];
    const result = mergeTimeline(comments, []);

    expect(result).toHaveLength(1);
    expect(result[0].type).toBe('comment');
  });

  it('returns empty array when both are empty', () => {
    expect(mergeTimeline([], [])).toEqual([]);
  });

  it('has correct discriminator types', () => {
    const comments = [createComment({ id: 'c1', created_at: '2026-01-01T12:00:00Z' })];
    const sessions = [createSession({ id: 's1', created_at: '2026-01-01T10:00:00Z' })];

    const result = mergeTimeline(comments, sessions);

    const commentItem = result.find((item): item is TimelineItem & { type: 'comment' } => item.type === 'comment');
    const sessionItem = result.find((item): item is TimelineItem & { type: 'agent_session' } => item.type === 'agent_session');

    expect(commentItem?.data.content).toBe('Test comment');
    expect(sessionItem?.data.agent_type).toBe(AgentType.claude_code);
  });
});
