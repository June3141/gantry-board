import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import * as commentsApi from '@/api/generated/endpoints/task-comments/task-comments';
import type { TaskComment } from '@/api/generated/model';
import { useAuthStore } from '@/stores/authStore';
import { CommentSection } from './CommentSection';

vi.mock('@/api/generated/endpoints/task-comments/task-comments', () => ({
  useListComments: vi.fn(),
  useCreateComment: vi.fn(),
  useUpdateComment: vi.fn(),
  useDeleteComment: vi.fn(),
}));

const mockComments: TaskComment[] = [
  {
    id: 'comment-1',
    task_id: 'task-1',
    user_id: 'user-1',
    user_name: 'Alice',
    content: 'First comment',
    created_at: '2026-01-01T00:00:00Z',
    updated_at: '2026-01-01T00:00:00Z',
  },
  {
    id: 'comment-2',
    task_id: 'task-1',
    user_id: 'user-2',
    user_name: 'Bob',
    content: 'Second comment',
    created_at: '2026-01-01T01:00:00Z',
    updated_at: '2026-01-01T01:00:00Z',
  },
];

const createQueryClient = () =>
  new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });

const renderWithProviders = (ui: React.ReactElement) => {
  const queryClient = createQueryClient();
  return render(<QueryClientProvider client={queryClient}>{ui}</QueryClientProvider>);
};

describe('CommentSection', () => {
  const mockCreateMutateAsync = vi.fn();
  const mockUpdateMutateAsync = vi.fn();
  const mockDeleteMutateAsync = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();

    useAuthStore.setState({
      user: {
        id: 'user-1',
        email: 'alice@test.com',
        name: 'Alice',
        created_at: '',
        updated_at: '',
      },
      isAuthenticated: true,
      isLoading: false,
    });

    vi.mocked(commentsApi.useListComments).mockReturnValue({
      data: mockComments,
      isLoading: false,
    } as unknown as ReturnType<typeof commentsApi.useListComments>);

    vi.mocked(commentsApi.useCreateComment).mockReturnValue({
      mutateAsync: mockCreateMutateAsync,
      isPending: false,
    } as unknown as ReturnType<typeof commentsApi.useCreateComment>);

    vi.mocked(commentsApi.useUpdateComment).mockReturnValue({
      mutateAsync: mockUpdateMutateAsync,
      isPending: false,
    } as unknown as ReturnType<typeof commentsApi.useUpdateComment>);

    vi.mocked(commentsApi.useDeleteComment).mockReturnValue({
      mutateAsync: mockDeleteMutateAsync,
      isPending: false,
    } as unknown as ReturnType<typeof commentsApi.useDeleteComment>);
  });

  it('renders comment list', () => {
    renderWithProviders(<CommentSection taskId="task-1" />);
    expect(screen.getByText('First comment')).toBeInTheDocument();
    expect(screen.getByText('Second comment')).toBeInTheDocument();
  });

  it('renders user names', () => {
    renderWithProviders(<CommentSection taskId="task-1" />);
    expect(screen.getByText('Alice')).toBeInTheDocument();
    expect(screen.getByText('Bob')).toBeInTheDocument();
  });

  it('renders empty state when no comments', () => {
    vi.mocked(commentsApi.useListComments).mockReturnValue({
      data: [],
      isLoading: false,
    } as unknown as ReturnType<typeof commentsApi.useListComments>);
    renderWithProviders(<CommentSection taskId="task-1" />);
    expect(screen.getByText(/no comments/i)).toBeInTheDocument();
  });

  it('submits a new comment', async () => {
    const user = userEvent.setup();
    mockCreateMutateAsync.mockResolvedValue({});
    renderWithProviders(<CommentSection taskId="task-1" />);

    const textarea = screen.getByPlaceholderText(/add a comment/i);
    await user.type(textarea, 'New comment');
    await user.click(screen.getByRole('button', { name: /post/i }));

    expect(mockCreateMutateAsync).toHaveBeenCalledWith({
      taskId: 'task-1',
      data: { content: 'New comment' },
    });
  });

  it('clears textarea after successful submission', async () => {
    const user = userEvent.setup();
    mockCreateMutateAsync.mockResolvedValue({});
    renderWithProviders(<CommentSection taskId="task-1" />);

    const textarea = screen.getByPlaceholderText(/add a comment/i);
    await user.type(textarea, 'New comment');
    await user.click(screen.getByRole('button', { name: /post/i }));

    expect(textarea).toHaveValue('');
  });

  it('disables post button when textarea is empty', () => {
    renderWithProviders(<CommentSection taskId="task-1" />);
    const button = screen.getByRole('button', { name: /post/i });
    expect(button).toBeDisabled();
  });

  it('shows edit and delete buttons for own comments', () => {
    renderWithProviders(<CommentSection taskId="task-1" />);
    // user-1 is Alice (current user) — should see edit/delete on comment-1
    const commentItems = screen.getAllByTestId('comment-item');
    const firstComment = commentItems[0];
    expect(firstComment.querySelector('[aria-label="Edit"]')).toBeInTheDocument();
    expect(firstComment.querySelector('[aria-label="Delete"]')).toBeInTheDocument();
  });

  it('does not show edit and delete buttons for other users comments', () => {
    renderWithProviders(<CommentSection taskId="task-1" />);
    // user-2 is Bob — current user is user-1, should NOT see edit/delete on comment-2
    const commentItems = screen.getAllByTestId('comment-item');
    const secondComment = commentItems[1];
    expect(secondComment.querySelector('[aria-label="Edit"]')).not.toBeInTheDocument();
    expect(secondComment.querySelector('[aria-label="Delete"]')).not.toBeInTheDocument();
  });

  it('enters edit mode on edit button click', async () => {
    const user = userEvent.setup();
    renderWithProviders(<CommentSection taskId="task-1" />);

    const editBtn = screen.getAllByTestId('comment-item')[0].querySelector('[aria-label="Edit"]');
    expect(editBtn).toBeInTheDocument();
    await user.click(editBtn as HTMLElement);

    expect(screen.getByDisplayValue('First comment')).toBeInTheDocument();
  });

  it('saves edited comment', async () => {
    const user = userEvent.setup();
    mockUpdateMutateAsync.mockResolvedValue({});
    renderWithProviders(<CommentSection taskId="task-1" />);

    const editBtn = screen.getAllByTestId('comment-item')[0].querySelector('[aria-label="Edit"]');
    expect(editBtn).toBeInTheDocument();
    await user.click(editBtn as HTMLElement);

    const input = screen.getByDisplayValue('First comment');
    await user.clear(input);
    await user.type(input, 'Updated comment');
    await user.click(screen.getByRole('button', { name: /save/i }));

    expect(mockUpdateMutateAsync).toHaveBeenCalledWith({
      taskId: 'task-1',
      commentId: 'comment-1',
      data: { content: 'Updated comment' },
    });
  });

  it('cancels editing', async () => {
    const user = userEvent.setup();
    renderWithProviders(<CommentSection taskId="task-1" />);

    const editBtn = screen.getAllByTestId('comment-item')[0].querySelector('[aria-label="Edit"]');
    expect(editBtn).toBeInTheDocument();
    await user.click(editBtn as HTMLElement);

    await user.click(screen.getByRole('button', { name: /cancel/i }));

    expect(screen.queryByDisplayValue('First comment')).not.toBeInTheDocument();
    expect(screen.getByText('First comment')).toBeInTheDocument();
  });

  it('deletes comment with confirmation', async () => {
    const user = userEvent.setup();
    mockDeleteMutateAsync.mockResolvedValue({});
    renderWithProviders(<CommentSection taskId="task-1" />);

    const deleteBtn = screen
      .getAllByTestId('comment-item')[0]
      .querySelector('[aria-label="Delete"]');
    expect(deleteBtn).toBeInTheDocument();
    await user.click(deleteBtn as HTMLElement);

    // Confirmation appears
    await user.click(screen.getByRole('button', { name: /confirm/i }));

    expect(mockDeleteMutateAsync).toHaveBeenCalledWith({
      taskId: 'task-1',
      commentId: 'comment-1',
    });
  });
});
