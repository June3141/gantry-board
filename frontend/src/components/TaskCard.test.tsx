import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import type { Task } from '../api/generated/model';
import { TaskPriority, TaskStatus } from '../api/generated/model';
import { useUiStore } from '../stores/uiStore';
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
});
