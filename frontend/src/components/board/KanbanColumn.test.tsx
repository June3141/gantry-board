import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import type { Task } from '@/api/generated/model';
import { TaskPriority, TaskStatus } from '@/api/generated/model';
import { useUiStore } from '@/stores/uiStore';
import { KanbanColumn } from './KanbanColumn';

const createMockTask = (overrides: Partial<Task> = {}): Task => ({
  id: 'task-1',
  project_id: 'project-1',
  title: 'Test Task',
  status: TaskStatus.todo,
  priority: TaskPriority.medium,
  position: 0,
  created_at: '2026-01-01T00:00:00Z',
  updated_at: '2026-01-01T00:00:00Z',
  ...overrides,
});

describe('KanbanColumn', () => {
  it('renders column title', () => {
    render(<KanbanColumn status={TaskStatus.todo} tasks={[]} />);

    expect(screen.getByText('To Do')).toBeInTheDocument();
  });

  it('renders correct title for each status', () => {
    const { rerender } = render(<KanbanColumn status={TaskStatus.backlog} tasks={[]} />);
    expect(screen.getByText('Backlog')).toBeInTheDocument();

    rerender(<KanbanColumn status={TaskStatus.todo} tasks={[]} />);
    expect(screen.getByText('To Do')).toBeInTheDocument();

    rerender(<KanbanColumn status={TaskStatus.in_progress} tasks={[]} />);
    expect(screen.getByText('In Progress')).toBeInTheDocument();

    rerender(<KanbanColumn status={TaskStatus.in_review} tasks={[]} />);
    expect(screen.getByText('In Review')).toBeInTheDocument();

    rerender(<KanbanColumn status={TaskStatus.done} tasks={[]} />);
    expect(screen.getByText('Done')).toBeInTheDocument();
  });

  it('renders task count', () => {
    const tasks = [createMockTask({ id: 'task-1' }), createMockTask({ id: 'task-2' })];
    render(<KanbanColumn status={TaskStatus.todo} tasks={tasks} />);

    expect(screen.getByText('2')).toBeInTheDocument();
  });

  it('renders zero count when no tasks', () => {
    render(<KanbanColumn status={TaskStatus.todo} tasks={[]} />);

    expect(screen.getByText('0')).toBeInTheDocument();
  });

  it('renders task cards for each task', () => {
    const tasks = [
      createMockTask({ id: 'task-1', title: 'First Task' }),
      createMockTask({ id: 'task-2', title: 'Second Task' }),
    ];
    render(<KanbanColumn status={TaskStatus.todo} tasks={tasks} />);

    expect(screen.getByText('First Task')).toBeInTheDocument();
    expect(screen.getByText('Second Task')).toBeInTheDocument();
  });

  it('renders empty state when no tasks', () => {
    render(<KanbanColumn status={TaskStatus.todo} tasks={[]} />);

    expect(screen.getByTestId('column-empty')).toBeInTheDocument();
  });

  it('renders add task button', () => {
    render(<KanbanColumn status={TaskStatus.todo} tasks={[]} />);

    expect(screen.getByRole('button', { name: /add task/i })).toBeInTheDocument();
  });

  it('opens task modal with column status on add task click', async () => {
    const user = userEvent.setup();
    const openTaskModal = vi.fn();
    useUiStore.setState({ openTaskModal });

    render(<KanbanColumn status={TaskStatus.in_progress} tasks={[]} />);
    await user.click(screen.getByRole('button', { name: /add task/i }));

    expect(openTaskModal).toHaveBeenCalledWith(TaskStatus.in_progress);
  });

  it('applies status-specific background color', () => {
    const { container, rerender } = render(<KanbanColumn status={TaskStatus.backlog} tasks={[]} />);
    const column = container.firstChild as HTMLElement;
    expect(column.className).toContain('bg-slate-50');

    rerender(<KanbanColumn status={TaskStatus.todo} tasks={[]} />);
    expect((container.firstChild as HTMLElement).className).toContain('bg-blue-50');

    rerender(<KanbanColumn status={TaskStatus.in_progress} tasks={[]} />);
    expect((container.firstChild as HTMLElement).className).toContain('bg-amber-50');

    rerender(<KanbanColumn status={TaskStatus.in_review} tasks={[]} />);
    expect((container.firstChild as HTMLElement).className).toContain('bg-purple-50');

    rerender(<KanbanColumn status={TaskStatus.done} tasks={[]} />);
    expect((container.firstChild as HTMLElement).className).toContain('bg-green-50');
  });

  it('applies status-specific badge color', () => {
    const { rerender } = render(<KanbanColumn status={TaskStatus.backlog} tasks={[]} />);

    const getBadge = () => screen.getByText('0');

    expect(getBadge().className).toContain('bg-slate-200');

    rerender(<KanbanColumn status={TaskStatus.todo} tasks={[]} />);
    expect(getBadge().className).toContain('bg-blue-200');

    rerender(<KanbanColumn status={TaskStatus.in_progress} tasks={[]} />);
    expect(getBadge().className).toContain('bg-amber-200');

    rerender(<KanbanColumn status={TaskStatus.in_review} tasks={[]} />);
    expect(getBadge().className).toContain('bg-purple-200');

    rerender(<KanbanColumn status={TaskStatus.done} tasks={[]} />);
    expect(getBadge().className).toContain('bg-green-200');
  });
});
