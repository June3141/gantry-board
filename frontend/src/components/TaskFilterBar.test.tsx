import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import type { ProjectMember } from '../api/generated/model';
import { MemberRole, TaskPriority } from '../api/generated/model';
import { useBoardStore } from '../stores/boardStore';
import { TaskFilterBar } from './TaskFilterBar';

const mockMembers: ProjectMember[] = [
  {
    user_id: 'user-1',
    user_name: 'Alice',
    user_email: 'alice@test.com',
    role: MemberRole.owner,
    project_id: 'proj-1',
    created_at: '2026-01-01T00:00:00Z',
  },
  {
    user_id: 'user-2',
    user_name: 'Bob',
    user_email: 'bob@test.com',
    role: MemberRole.member,
    project_id: 'proj-1',
    created_at: '2026-01-01T00:00:00Z',
  },
];

describe('TaskFilterBar', () => {
  beforeEach(() => {
    useBoardStore.getState().clearFilters();
  });

  it('renders search input', () => {
    render(<TaskFilterBar members={mockMembers} />);
    expect(screen.getByPlaceholderText(/search/i)).toBeInTheDocument();
  });

  it('updates searchText in boardStore on input', async () => {
    const user = userEvent.setup();
    render(<TaskFilterBar members={mockMembers} />);

    await user.type(screen.getByPlaceholderText(/search/i), 'bug');

    expect(useBoardStore.getState().searchText).toBe('bug');
  });

  it('renders assignee filter dropdown with members and Unassigned', async () => {
    const user = userEvent.setup();
    render(<TaskFilterBar members={mockMembers} />);

    await user.click(screen.getByRole('button', { name: /assignee/i }));

    const dropdown = screen.getByTestId('assignee-dropdown');
    expect(within(dropdown).getByLabelText('Unassigned')).toBeInTheDocument();
    expect(within(dropdown).getByLabelText('Alice')).toBeInTheDocument();
    expect(within(dropdown).getByLabelText('Bob')).toBeInTheDocument();
  });

  it('updates assigneeFilter when checkbox is toggled', async () => {
    const user = userEvent.setup();
    render(<TaskFilterBar members={mockMembers} />);

    await user.click(screen.getByRole('button', { name: /assignee/i }));
    await user.click(screen.getByLabelText('Alice'));

    expect(useBoardStore.getState().assigneeFilter).toEqual(['user-1']);
  });

  it('renders priority filter with all levels', async () => {
    const user = userEvent.setup();
    render(<TaskFilterBar members={mockMembers} />);

    await user.click(screen.getByRole('button', { name: /priority/i }));

    const dropdown = screen.getByTestId('priority-dropdown');
    expect(within(dropdown).getByLabelText('Low')).toBeInTheDocument();
    expect(within(dropdown).getByLabelText('Medium')).toBeInTheDocument();
    expect(within(dropdown).getByLabelText('High')).toBeInTheDocument();
    expect(within(dropdown).getByLabelText('Urgent')).toBeInTheDocument();
  });

  it('updates priorityFilter when checkbox is toggled', async () => {
    const user = userEvent.setup();
    render(<TaskFilterBar members={mockMembers} />);

    await user.click(screen.getByRole('button', { name: /priority/i }));
    await user.click(screen.getByLabelText('High'));

    expect(useBoardStore.getState().priorityFilter).toEqual([TaskPriority.high]);
  });

  it('does not show Clear all when no filters active', () => {
    render(<TaskFilterBar members={mockMembers} />);
    expect(screen.queryByRole('button', { name: /clear all/i })).not.toBeInTheDocument();
  });

  it('shows Clear all when filters are active', () => {
    useBoardStore.getState().setSearchText('test');
    render(<TaskFilterBar members={mockMembers} />);
    expect(screen.getByRole('button', { name: /clear all/i })).toBeInTheDocument();
  });

  it('resets all filters on Clear all click', async () => {
    const user = userEvent.setup();
    useBoardStore.getState().setSearchText('test');
    useBoardStore.getState().setAssigneeFilter(['user-1']);
    useBoardStore.getState().setPriorityFilter([TaskPriority.high]);
    render(<TaskFilterBar members={mockMembers} />);

    await user.click(screen.getByRole('button', { name: /clear all/i }));

    const state = useBoardStore.getState();
    expect(state.searchText).toBe('');
    expect(state.assigneeFilter).toEqual([]);
    expect(state.priorityFilter).toEqual([]);
  });
});
