import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import * as messagesApi from '@/api/generated/endpoints/project-messages/project-messages';
import type { ProjectMessage } from '@/api/generated/model';
import { useAuthStore } from '@/stores/authStore';
import { useUiStore } from '@/stores/uiStore';
import { ProjectChatPanel } from './ProjectChatPanel';

vi.mock('@/api/generated/endpoints/project-messages/project-messages', () => ({
  useListMessages: vi.fn(),
  useCreateMessage: vi.fn(),
  useDeleteMessage: vi.fn(),
  getListMessagesQueryKey: vi.fn(() => ['/api/projects/project-1/messages']),
}));

const now = new Date().toISOString();

const mockMessages: ProjectMessage[] = [
  {
    id: 'msg-1',
    project_id: 'project-1',
    user_id: 'user-1',
    user_name: 'Alice',
    content: 'Hello team!',
    created_at: now,
  },
  {
    id: 'msg-2',
    project_id: 'project-1',
    user_id: 'user-2',
    user_name: 'Bob',
    content: 'Hi Alice!',
    created_at: now,
  },
];

const createQueryClient = () => new QueryClient({ defaultOptions: { queries: { retry: false } } });

const renderWithProviders = (ui: React.ReactElement) => {
  const queryClient = createQueryClient();
  return render(<QueryClientProvider client={queryClient}>{ui}</QueryClientProvider>);
};

describe('ProjectChatPanel', () => {
  const mockCreateMutateAsync = vi.fn();
  const mockDeleteMutateAsync = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();

    useAuthStore.setState({
      user: {
        id: 'user-1',
        email: 'alice@test.com',
        name: 'Alice',
        is_admin: false,
        created_at: '',
        updated_at: '',
      },
      isAuthenticated: true,
      isLoading: false,
    });

    useUiStore.setState({ isProjectChatOpen: false });

    vi.mocked(messagesApi.useListMessages).mockReturnValue({
      data: mockMessages,
      isLoading: false,
    } as unknown as ReturnType<typeof messagesApi.useListMessages>);

    vi.mocked(messagesApi.useCreateMessage).mockReturnValue({
      mutateAsync: mockCreateMutateAsync,
      isPending: false,
    } as unknown as ReturnType<typeof messagesApi.useCreateMessage>);

    vi.mocked(messagesApi.useDeleteMessage).mockReturnValue({
      mutateAsync: mockDeleteMutateAsync,
      isPending: false,
    } as unknown as ReturnType<typeof messagesApi.useDeleteMessage>);
  });

  it('does not render when panel is closed', () => {
    renderWithProviders(<ProjectChatPanel projectId="project-1" />);
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it('renders message list when open', () => {
    useUiStore.setState({ isProjectChatOpen: true });
    renderWithProviders(<ProjectChatPanel projectId="project-1" />);
    expect(screen.getByText('Hello team!')).toBeInTheDocument();
    expect(screen.getByText('Hi Alice!')).toBeInTheDocument();
  });

  it('displays user names', () => {
    useUiStore.setState({ isProjectChatOpen: true });
    renderWithProviders(<ProjectChatPanel projectId="project-1" />);
    expect(screen.getByText('Alice')).toBeInTheDocument();
    expect(screen.getByText('Bob')).toBeInTheDocument();
  });

  it('shows empty state when no messages', () => {
    vi.mocked(messagesApi.useListMessages).mockReturnValue({
      data: [],
      isLoading: false,
    } as unknown as ReturnType<typeof messagesApi.useListMessages>);

    useUiStore.setState({ isProjectChatOpen: true });
    renderWithProviders(<ProjectChatPanel projectId="project-1" />);
    expect(screen.getByText(/no messages/i)).toBeInTheDocument();
  });

  it('sends a message when form is submitted', async () => {
    const user = userEvent.setup();
    mockCreateMutateAsync.mockResolvedValue({});
    useUiStore.setState({ isProjectChatOpen: true });
    renderWithProviders(<ProjectChatPanel projectId="project-1" />);

    const input = screen.getByPlaceholderText(/type a message/i);
    await user.type(input, 'New message');
    await user.click(screen.getByRole('button', { name: /send/i }));

    expect(mockCreateMutateAsync).toHaveBeenCalledWith({
      projectId: 'project-1',
      data: { content: 'New message' },
    });
  });

  it('clears input after sending', async () => {
    const user = userEvent.setup();
    mockCreateMutateAsync.mockResolvedValue({});
    useUiStore.setState({ isProjectChatOpen: true });
    renderWithProviders(<ProjectChatPanel projectId="project-1" />);

    const input = screen.getByPlaceholderText(/type a message/i);
    await user.type(input, 'New message');
    await user.click(screen.getByRole('button', { name: /send/i }));

    expect(input).toHaveValue('');
  });

  it('disables send button when input is empty', () => {
    useUiStore.setState({ isProjectChatOpen: true });
    renderWithProviders(<ProjectChatPanel projectId="project-1" />);

    const sendButton = screen.getByRole('button', { name: /send/i });
    expect(sendButton).toBeDisabled();
  });

  it('shows delete button for own messages', () => {
    useUiStore.setState({ isProjectChatOpen: true });
    renderWithProviders(<ProjectChatPanel projectId="project-1" />);

    const deleteButtons = screen.getAllByRole('button', { name: /delete/i });
    // Only Alice's message (msg-1) should have a delete button
    expect(deleteButtons).toHaveLength(1);
  });

  it('deletes a message with confirmation', async () => {
    const user = userEvent.setup();
    mockDeleteMutateAsync.mockResolvedValue({});
    useUiStore.setState({ isProjectChatOpen: true });
    renderWithProviders(<ProjectChatPanel projectId="project-1" />);

    await user.click(screen.getByRole('button', { name: /delete/i }));
    await user.click(screen.getByRole('button', { name: /confirm/i }));

    expect(mockDeleteMutateAsync).toHaveBeenCalledWith({
      projectId: 'project-1',
      messageId: 'msg-1',
    });
  });

  it('closes on Escape key', async () => {
    const user = userEvent.setup();
    useUiStore.setState({ isProjectChatOpen: true });
    renderWithProviders(<ProjectChatPanel projectId="project-1" />);

    await user.keyboard('{Escape}');
    expect(useUiStore.getState().isProjectChatOpen).toBe(false);
  });

  it('closes on backdrop click', async () => {
    const user = userEvent.setup();
    useUiStore.setState({ isProjectChatOpen: true });
    renderWithProviders(<ProjectChatPanel projectId="project-1" />);

    const overlay = document.querySelector('[data-slot="dialog-overlay"]') as HTMLElement;
    await user.click(overlay);
    expect(useUiStore.getState().isProjectChatOpen).toBe(false);
  });
});
