import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { act, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import * as projectsApi from '@/api/generated/endpoints/projects/projects';
import { useUiStore } from '@/stores/uiStore';
import { ProjectCreateDialog } from './ProjectCreateDialog';

vi.mock('@/api/generated/endpoints/projects/projects', () => ({
  useCreateProject: vi.fn(),
}));

const createQueryClient = () =>
  new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });

const renderWithProviders = (ui: React.ReactElement) => {
  const queryClient = createQueryClient();
  return render(<QueryClientProvider client={queryClient}>{ui}</QueryClientProvider>);
};

describe('ProjectCreateDialog', () => {
  const mockMutateAsync = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
    mockMutateAsync.mockResolvedValue({});
    vi.mocked(projectsApi.useCreateProject).mockReturnValue({
      mutateAsync: mockMutateAsync,
      isPending: false,
    } as unknown as ReturnType<typeof projectsApi.useCreateProject>);
    useUiStore.setState({
      isTaskModalOpen: false,
      defaultStatus: null,
      isProjectModalOpen: false,
    });
  });

  it('does not render when modal is closed', () => {
    renderWithProviders(<ProjectCreateDialog />);
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it('renders form when modal is open', () => {
    useUiStore.setState({ isProjectModalOpen: true });
    renderWithProviders(<ProjectCreateDialog />);

    expect(screen.getByRole('dialog')).toBeInTheDocument();
    expect(screen.getByLabelText(/name/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/description/i)).toBeInTheDocument();
  });

  it('submits form with correct data', async () => {
    const user = userEvent.setup();
    useUiStore.setState({ isProjectModalOpen: true });
    renderWithProviders(<ProjectCreateDialog />);

    await user.type(screen.getByLabelText(/name/i), 'My Project');
    await user.type(screen.getByLabelText(/description/i), 'Project desc');
    await user.click(screen.getByRole('button', { name: /create/i }));

    expect(mockMutateAsync).toHaveBeenCalledWith({
      data: {
        name: 'My Project',
        description: 'Project desc',
      },
    });
  });

  it('closes modal after successful submission', async () => {
    const user = userEvent.setup();
    useUiStore.setState({ isProjectModalOpen: true });
    renderWithProviders(<ProjectCreateDialog />);

    await user.type(screen.getByLabelText(/name/i), 'My Project');
    await user.click(screen.getByRole('button', { name: /create/i }));

    expect(useUiStore.getState().isProjectModalOpen).toBe(false);
  });

  it('closes modal on cancel', async () => {
    const user = userEvent.setup();
    useUiStore.setState({ isProjectModalOpen: true });
    renderWithProviders(<ProjectCreateDialog />);

    await user.click(screen.getByRole('button', { name: /cancel/i }));

    expect(useUiStore.getState().isProjectModalOpen).toBe(false);
  });

  it('does not submit when name is empty', async () => {
    const user = userEvent.setup();
    useUiStore.setState({ isProjectModalOpen: true });
    renderWithProviders(<ProjectCreateDialog />);

    await user.click(screen.getByRole('button', { name: /create/i }));

    expect(mockMutateAsync).not.toHaveBeenCalled();
  });

  it('resets form when reopened', async () => {
    const user = userEvent.setup();
    useUiStore.setState({ isProjectModalOpen: true });
    renderWithProviders(<ProjectCreateDialog />);

    await user.type(screen.getByLabelText(/name/i), 'Some name');
    await user.click(screen.getByRole('button', { name: /cancel/i }));

    act(() => {
      useUiStore.setState({ isProjectModalOpen: true });
    });

    expect(screen.getByLabelText(/name/i)).toHaveValue('');
  });
});
