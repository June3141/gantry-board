import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';

import { useToastStore } from '../stores/toastStore';
import { ToastContainer } from './ToastContainer';

describe('ToastContainer', () => {
  afterEach(() => {
    useToastStore.setState({ toasts: [] });
  });

  it('renders nothing when no toasts', () => {
    const { container } = render(<ToastContainer />);
    expect(container.querySelector('[data-testid="toast-container"]')?.children).toHaveLength(0);
  });

  it('renders toast message', () => {
    useToastStore.setState({
      toasts: [{ id: '1', type: 'success', message: 'Saved successfully' }],
    });
    render(<ToastContainer />);
    expect(screen.getByText('Saved successfully')).toBeInTheDocument();
  });

  it('renders multiple toasts', () => {
    useToastStore.setState({
      toasts: [
        { id: '1', type: 'success', message: 'First toast' },
        { id: '2', type: 'error', message: 'Second toast' },
      ],
    });
    render(<ToastContainer />);
    expect(screen.getByText('First toast')).toBeInTheDocument();
    expect(screen.getByText('Second toast')).toBeInTheDocument();
  });

  it('dismisses toast when close button is clicked', async () => {
    const user = userEvent.setup();
    useToastStore.setState({
      toasts: [{ id: '1', type: 'info', message: 'Dismiss me' }],
    });
    render(<ToastContainer />);

    await user.click(screen.getByRole('button', { name: /dismiss/i }));
    expect(useToastStore.getState().toasts).toHaveLength(0);
  });

  it('applies green styling for success toast', () => {
    useToastStore.setState({
      toasts: [{ id: '1', type: 'success', message: 'Success msg' }],
    });
    render(<ToastContainer />);
    const toast = screen.getByText('Success msg').closest('[data-testid="toast-item"]');
    expect(toast).toHaveClass('bg-green-50');
  });

  it('applies red styling for error toast', () => {
    useToastStore.setState({
      toasts: [{ id: '1', type: 'error', message: 'Error msg' }],
    });
    render(<ToastContainer />);
    const toast = screen.getByText('Error msg').closest('[data-testid="toast-item"]');
    expect(toast).toHaveClass('bg-red-50');
  });

  it('applies blue styling for info toast', () => {
    useToastStore.setState({
      toasts: [{ id: '1', type: 'info', message: 'Info msg' }],
    });
    render(<ToastContainer />);
    const toast = screen.getByText('Info msg').closest('[data-testid="toast-item"]');
    expect(toast).toHaveClass('bg-blue-50');
  });
});
