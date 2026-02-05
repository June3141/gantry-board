import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { TaskStatus } from '../api/generated/model';
import * as tasksApi from '../api/generated/endpoints/tasks/tasks';
import { useUiStore } from '../stores/uiStore';
import { TaskCreateDialog } from './TaskCreateDialog';

vi.mock('../api/generated/endpoints/tasks/tasks', () => ({
  useCreateTask: vi.fn(),
}));

const createQueryClient = () =>
  new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });

const renderWithProviders = (ui: React.ReactElement) => {
  const queryClient = createQueryClient();
  return render(<QueryClientProvider client={queryClient}>{ui}</QueryClientProvider>);
};

describe('TaskCreateDialog', () => {
  const mockMutateAsync = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
    mockMutateAsync.mockResolvedValue({});
    vi.mocked(tasksApi.useCreateTask).mockReturnValue({
      mutateAsync: mockMutateAsync,
      isPending: false,
    } as unknown as ReturnType<typeof tasksApi.useCreateTask>);
    useUiStore.setState({
      isTaskModalOpen: false,
      defaultStatus: null,
      isProjectModalOpen: false,
    });
  });

  it('does not render when modal is closed', () => {
    renderWithProviders(<TaskCreateDialog projectId="proj-1" />);
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it('renders form when modal is open', () => {
    useUiStore.setState({ isTaskModalOpen: true });
    renderWithProviders(<TaskCreateDialog projectId="proj-1" />);

    expect(screen.getByRole('dialog')).toBeInTheDocument();
    expect(screen.getByLabelText(/title/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/description/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/status/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/priority/i)).toBeInTheDocument();
  });

  it('pre-fills status from defaultStatus', () => {
    useUiStore.setState({ isTaskModalOpen: true, defaultStatus: TaskStatus.in_progress });
    renderWithProviders(<TaskCreateDialog projectId="proj-1" />);

    const statusSelect = screen.getByLabelText(/status/i) as HTMLSelectElement;
    expect(statusSelect.value).toBe('in_progress');
  });

  it('defaults status to backlog when no defaultStatus', () => {
    useUiStore.setState({ isTaskModalOpen: true, defaultStatus: null });
    renderWithProviders(<TaskCreateDialog projectId="proj-1" />);

    const statusSelect = screen.getByLabelText(/status/i) as HTMLSelectElement;
    expect(statusSelect.value).toBe('backlog');
  });

  it('submits form with correct data', async () => {
    const user = userEvent.setup();
    useUiStore.setState({ isTaskModalOpen: true, defaultStatus: TaskStatus.todo });
    renderWithProviders(<TaskCreateDialog projectId="proj-1" />);

    await user.type(screen.getByLabelText(/title/i), 'New Task');
    await user.type(screen.getByLabelText(/description/i), 'Task description');
    await user.selectOptions(screen.getByLabelText(/priority/i), 'high');
    await user.click(screen.getByRole('button', { name: /create/i }));

    expect(mockMutateAsync).toHaveBeenCalledWith({
      data: {
        project_id: 'proj-1',
        title: 'New Task',
        description: 'Task description',
        status: 'todo',
        priority: 'high',
      },
    });
  });

  it('closes modal after successful submission', async () => {
    const user = userEvent.setup();
    useUiStore.setState({ isTaskModalOpen: true });
    renderWithProviders(<TaskCreateDialog projectId="proj-1" />);

    await user.type(screen.getByLabelText(/title/i), 'New Task');
    await user.click(screen.getByRole('button', { name: /create/i }));

    expect(useUiStore.getState().isTaskModalOpen).toBe(false);
  });

  it('closes modal on cancel', async () => {
    const user = userEvent.setup();
    useUiStore.setState({ isTaskModalOpen: true });
    renderWithProviders(<TaskCreateDialog projectId="proj-1" />);

    await user.click(screen.getByRole('button', { name: /cancel/i }));

    expect(useUiStore.getState().isTaskModalOpen).toBe(false);
  });

  it('does not submit when title is empty', async () => {
    const user = userEvent.setup();
    useUiStore.setState({ isTaskModalOpen: true });
    renderWithProviders(<TaskCreateDialog projectId="proj-1" />);

    await user.click(screen.getByRole('button', { name: /create/i }));

    expect(mockMutateAsync).not.toHaveBeenCalled();
  });

  it('resets form when reopened', async () => {
    const user = userEvent.setup();
    useUiStore.setState({ isTaskModalOpen: true });
    renderWithProviders(<TaskCreateDialog projectId="proj-1" />);

    await user.type(screen.getByLabelText(/title/i), 'Some title');
    await user.click(screen.getByRole('button', { name: /cancel/i }));

    // Reopen
    useUiStore.setState({ isTaskModalOpen: true });

    expect(screen.getByLabelText(/title/i)).toHaveValue('');
  });
});
