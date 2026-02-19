import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { http, HttpResponse } from 'msw';
import React from 'react';
import { beforeEach, describe, expect, it } from 'vitest';
import type { Task } from '@/api/generated/model';
import { useUiStore } from '@/stores/uiStore';
import { server } from '@/test/mocks/server';
import { TaskDetailModal } from './TaskDetailModal';

const API = 'http://localhost:3000';

function renderModal() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });
  return render(
    React.createElement(
      QueryClientProvider,
      { client: queryClient },
      React.createElement(TaskDetailModal),
    ),
  );
}

function openModal() {
  useUiStore.setState({ selectedTaskId: 'task-1', isTaskDetailOpen: true });
}

/** Wait for the task title to appear (data loaded). */
async function waitForLoaded() {
  await waitFor(() => {
    expect(screen.getByText('Test Task')).toBeInTheDocument();
  });
}

describe('TaskDetailModal (MSW)', () => {
  beforeEach(() => {
    useUiStore.setState({ selectedTaskId: null, isTaskDetailOpen: false });
  });

  describe('inline editing', () => {
    it('enters title edit mode on click', async () => {
      const user = userEvent.setup();
      openModal();
      renderModal();
      await waitForLoaded();

      await user.click(screen.getByText('Test Task'));
      expect(screen.getByDisplayValue('Test Task')).toBeInTheDocument();
    });

    it('saves title on blur', async () => {
      let capturedBody: Partial<Task> | undefined;
      server.use(
        http.patch(`${API}/api/tasks/:id`, async ({ request }) => {
          capturedBody = (await request.json()) as Partial<Task>;
          return HttpResponse.json({ id: 'task-1', ...capturedBody });
        }),
      );

      const user = userEvent.setup();
      openModal();
      renderModal();
      await waitForLoaded();

      await user.click(screen.getByText('Test Task'));
      const input = screen.getByDisplayValue('Test Task');
      await user.clear(input);
      await user.type(input, 'Updated Title');
      await user.tab();

      await waitFor(() => {
        expect(capturedBody).toEqual({ title: 'Updated Title' });
      });
    });

    it('does not send request for empty title', async () => {
      let patchCalled = false;
      server.use(
        http.patch(`${API}/api/tasks/:id`, async () => {
          patchCalled = true;
          return HttpResponse.json({});
        }),
      );

      const user = userEvent.setup();
      openModal();
      renderModal();
      await waitForLoaded();

      await user.click(screen.getByText('Test Task'));
      const input = screen.getByDisplayValue('Test Task');
      await user.clear(input);
      await user.tab();

      // Give the async handler a chance to fire (it shouldn't)
      await new Promise((r) => setTimeout(r, 100));
      expect(patchCalled).toBe(false);
    });

    it('enters description edit mode on click', async () => {
      const user = userEvent.setup();
      openModal();
      renderModal();
      await waitForLoaded();

      await user.click(screen.getByText('Test description'));
      expect(screen.getByDisplayValue('Test description')).toBeInTheDocument();
    });

    it('saves description on blur', async () => {
      let capturedBody: Partial<Task> | undefined;
      server.use(
        http.patch(`${API}/api/tasks/:id`, async ({ request }) => {
          capturedBody = (await request.json()) as Partial<Task>;
          return HttpResponse.json({ id: 'task-1', ...capturedBody });
        }),
      );

      const user = userEvent.setup();
      openModal();
      renderModal();
      await waitForLoaded();

      await user.click(screen.getByText('Test description'));
      const textarea = screen.getByDisplayValue('Test description');
      await user.clear(textarea);
      await user.type(textarea, 'Updated description');
      await user.tab();

      await waitFor(() => {
        expect(capturedBody).toEqual({ description: 'Updated description' });
      });
    });

    it('updates status via select', async () => {
      let capturedBody: Partial<Task> | undefined;
      server.use(
        http.patch(`${API}/api/tasks/:id`, async ({ request }) => {
          capturedBody = (await request.json()) as Partial<Task>;
          return HttpResponse.json({ id: 'task-1', ...capturedBody });
        }),
      );

      const user = userEvent.setup();
      openModal();
      renderModal();
      await waitForLoaded();

      await user.selectOptions(screen.getByLabelText(/status/i), 'in_progress');

      await waitFor(() => {
        expect(capturedBody).toEqual({ status: 'in_progress' });
      });
    });

    it('updates priority via select', async () => {
      let capturedBody: Partial<Task> | undefined;
      server.use(
        http.patch(`${API}/api/tasks/:id`, async ({ request }) => {
          capturedBody = (await request.json()) as Partial<Task>;
          return HttpResponse.json({ id: 'task-1', ...capturedBody });
        }),
      );

      const user = userEvent.setup();
      openModal();
      renderModal();
      await waitForLoaded();

      await user.selectOptions(screen.getByLabelText(/priority/i), 'high');

      await waitFor(() => {
        expect(capturedBody).toEqual({ priority: 'high' });
      });
    });
  });

  describe('assignee', () => {
    it('displays assignee select with members', async () => {
      openModal();
      renderModal();
      await waitForLoaded();

      const select = screen.getByLabelText(/assignee/i) as HTMLSelectElement;
      await waitFor(() => {
        const options = Array.from(select.options);
        expect(options.some((o) => o.text === 'Alice')).toBe(true);
        expect(options.some((o) => o.text === 'Bob')).toBe(true);
      });
    });

    it('has Unassigned option', async () => {
      openModal();
      renderModal();
      await waitForLoaded();

      const select = screen.getByLabelText(/assignee/i) as HTMLSelectElement;
      const options = Array.from(select.options);
      expect(options.some((o) => o.text === 'Unassigned')).toBe(true);
    });

    it('calls update when assignee is changed', async () => {
      let capturedBody: Record<string, unknown> | undefined;
      server.use(
        http.patch(`${API}/api/tasks/:id`, async ({ request }) => {
          capturedBody = (await request.json()) as Record<string, unknown>;
          return HttpResponse.json({ id: 'task-1', ...capturedBody });
        }),
      );

      const user = userEvent.setup();
      openModal();
      renderModal();
      await waitForLoaded();

      // Wait for members to load
      await waitFor(() => {
        const select = screen.getByLabelText(/assignee/i) as HTMLSelectElement;
        expect(Array.from(select.options).some((o) => o.text === 'Bob')).toBe(true);
      });

      await user.selectOptions(screen.getByLabelText(/assignee/i), 'user-2');

      await waitFor(() => {
        expect(capturedBody).toEqual({ assigned_to: 'user-2' });
      });
    });

    it('sends null when Unassigned is selected', async () => {
      // Serve a task with an assignee
      server.use(
        http.get(`${API}/api/tasks/:id`, () => {
          return HttpResponse.json({
            id: 'task-1',
            project_id: 'project-1',
            title: 'Test Task',
            description: 'Test description',
            status: 'todo',
            priority: 'medium',
            position: 0,
            assigned_to: 'user-1',
            created_at: '2026-01-01T00:00:00Z',
            updated_at: '2026-01-01T00:00:00Z',
          });
        }),
      );

      let capturedBody: Record<string, unknown> | undefined;
      server.use(
        http.patch(`${API}/api/tasks/:id`, async ({ request }) => {
          capturedBody = (await request.json()) as Record<string, unknown>;
          return HttpResponse.json({ id: 'task-1', ...capturedBody });
        }),
      );

      const user = userEvent.setup();
      openModal();
      renderModal();
      await waitForLoaded();

      // Wait for members to load
      await waitFor(() => {
        const select = screen.getByLabelText(/assignee/i) as HTMLSelectElement;
        expect(Array.from(select.options).some((o) => o.text === 'Alice')).toBe(true);
      });

      await user.selectOptions(screen.getByLabelText(/assignee/i), '');

      await waitFor(() => {
        expect(capturedBody).toEqual({ assigned_to: null });
      });
    });
  });

  describe('delete', () => {
    it('shows delete button', async () => {
      openModal();
      renderModal();
      await waitForLoaded();

      expect(screen.getByRole('button', { name: /delete/i })).toBeInTheDocument();
    });

    it('shows confirmation dialog when delete is clicked', async () => {
      const user = userEvent.setup();
      openModal();
      renderModal();
      await waitForLoaded();

      await user.click(screen.getByRole('button', { name: /delete/i }));
      expect(screen.getByText(/are you sure/i)).toBeInTheDocument();
    });

    it('cancels deletion when cancel is clicked', async () => {
      let deleteCalled = false;
      server.use(
        http.delete(`${API}/api/tasks/:id`, () => {
          deleteCalled = true;
          return new HttpResponse(null, { status: 204 });
        }),
      );

      const user = userEvent.setup();
      openModal();
      renderModal();
      await waitForLoaded();

      await user.click(screen.getByRole('button', { name: /delete/i }));
      await user.click(screen.getByRole('button', { name: /cancel/i }));

      expect(deleteCalled).toBe(false);
      expect(screen.queryByText(/are you sure/i)).not.toBeInTheDocument();
    });

    it('deletes task and closes modal on confirm', async () => {
      let deleteCalled = false;
      server.use(
        http.delete(`${API}/api/tasks/:id`, () => {
          deleteCalled = true;
          return new HttpResponse(null, { status: 204 });
        }),
      );

      const user = userEvent.setup();
      openModal();
      renderModal();
      await waitForLoaded();

      await user.click(screen.getByRole('button', { name: /delete/i }));
      await user.click(screen.getByRole('button', { name: /confirm/i }));

      await waitFor(() => {
        expect(deleteCalled).toBe(true);
      });
      await waitFor(() => {
        expect(useUiStore.getState().isTaskDetailOpen).toBe(false);
      });
    });
  });
});
