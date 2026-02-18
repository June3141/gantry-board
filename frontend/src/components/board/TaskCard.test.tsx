import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import type { ProjectMember, Task } from '@/api/generated/model';
import { MemberRole, TaskPriority, TaskStatus } from '@/api/generated/model';
import { useUiStore } from '@/stores/uiStore';
import { TaskCard } from './TaskCard';

const createMockTask = (overrides: Partial<Task> = {}): Task => ({
  id: 'task-1',
  project_id: 'project-1',
  title: 'Test Task',
  description: 'Test description',
  status: TaskStatus.todo,
  priority: TaskPriority.medium,
  position: 0,
  created_at: '2026-01-01T00:00:00Z',
  updated_at: '2026-01-01T00:00:00Z',
  ...overrides,
});

describe('TaskCard', () => {
  beforeEach(() => {
    useUiStore.setState({
      selectedTaskId: null,
      isTaskDetailOpen: false,
    });
  });

  it('renders task title', () => {
    const task = createMockTask({ title: 'My Task Title' });
    render(<TaskCard task={task} />);

    expect(screen.getByText('My Task Title')).toBeInTheDocument();
  });

  it('renders task description when provided', () => {
    const task = createMockTask({ description: 'A detailed description' });
    render(<TaskCard task={task} />);

    expect(screen.getByText('A detailed description')).toBeInTheDocument();
  });

  it('does not render description when not provided', () => {
    const task = createMockTask({ description: undefined });
    render(<TaskCard task={task} />);

    expect(screen.queryByTestId('task-description')).not.toBeInTheDocument();
  });

  it('renders priority badge', () => {
    const task = createMockTask({ priority: TaskPriority.urgent });
    render(<TaskCard task={task} />);

    expect(screen.getByText('urgent')).toBeInTheDocument();
  });

  it('applies urgent priority styling', () => {
    const task = createMockTask({ priority: TaskPriority.urgent });
    render(<TaskCard task={task} />);

    const badge = screen.getByText('urgent');
    expect(badge).toHaveClass('bg-red-100');
  });

  it('applies high priority styling', () => {
    const task = createMockTask({ priority: TaskPriority.high });
    render(<TaskCard task={task} />);

    const badge = screen.getByText('high');
    expect(badge).toHaveClass('bg-orange-100');
  });

  it('applies medium priority styling', () => {
    const task = createMockTask({ priority: TaskPriority.medium });
    render(<TaskCard task={task} />);

    const badge = screen.getByText('medium');
    expect(badge).toHaveClass('bg-yellow-100');
  });

  it('applies low priority styling', () => {
    const task = createMockTask({ priority: TaskPriority.low });
    render(<TaskCard task={task} />);

    const badge = screen.getByText('low');
    expect(badge).toHaveClass('bg-gray-100');
  });

  it('opens task detail on click', async () => {
    const user = userEvent.setup();
    const task = createMockTask({ id: 'task-42' });
    render(<TaskCard task={task} />);

    await user.click(screen.getByText('Test Task'));

    expect(useUiStore.getState().selectedTaskId).toBe('task-42');
    expect(useUiStore.getState().isTaskDetailOpen).toBe(true);
  });

  it('has pointer cursor', () => {
    const task = createMockTask();
    render(<TaskCard task={task} />);

    const card = screen.getByText('Test Task').closest('[data-testid="task-card"]');
    expect(card).toHaveClass('cursor-pointer');
  });

  describe('assignee display', () => {
    const mockMembers: ProjectMember[] = [
      {
        user_id: 'user-1',
        user_name: 'Alice Smith',
        user_email: 'alice@test.com',
        role: MemberRole.owner,
        project_id: 'project-1',
        created_at: '2026-01-01T00:00:00Z',
      },
      {
        user_id: 'user-2',
        user_name: 'Bob',
        user_email: 'bob@test.com',
        role: MemberRole.member,
        project_id: 'project-1',
        created_at: '2026-01-01T00:00:00Z',
      },
    ];

    it('displays assignee initials when assigned_to is set', () => {
      const task = createMockTask({ assigned_to: 'user-1' });
      render(<TaskCard task={task} members={mockMembers} />);

      const avatar = screen.getByTestId('assignee-avatar');
      expect(avatar).toBeInTheDocument();
      expect(avatar).toHaveTextContent('AS');
    });

    it('does not display assignee when assigned_to is null', () => {
      const task = createMockTask({ assigned_to: null });
      render(<TaskCard task={task} members={mockMembers} />);

      expect(screen.queryByTestId('assignee-avatar')).not.toBeInTheDocument();
    });

    it('does not display assignee when assigned_to is undefined', () => {
      const task = createMockTask();
      render(<TaskCard task={task} members={mockMembers} />);

      expect(screen.queryByTestId('assignee-avatar')).not.toBeInTheDocument();
    });

    it('shows single initial for single-word name', () => {
      const task = createMockTask({ assigned_to: 'user-2' });
      render(<TaskCard task={task} members={mockMembers} />);

      const avatar = screen.getByTestId('assignee-avatar');
      expect(avatar).toHaveTextContent('B');
    });

    it('shows fallback when member not found', () => {
      const task = createMockTask({ assigned_to: 'unknown-user' });
      render(<TaskCard task={task} members={mockMembers} />);

      const avatar = screen.getByTestId('assignee-avatar');
      expect(avatar).toHaveTextContent('?');
    });

    it('does not crash when members is undefined', () => {
      const task = createMockTask({ assigned_to: 'user-1' });
      render(<TaskCard task={task} />);

      const avatar = screen.getByTestId('assignee-avatar');
      expect(avatar).toHaveTextContent('?');
    });
  });
});
