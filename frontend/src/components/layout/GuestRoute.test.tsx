import { render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { describe, expect, it } from 'vitest';
import { useAuthStore } from '@/stores/authStore';
import { GuestRoute } from './GuestRoute';

const renderWithRouter = (ui: React.ReactElement, { route = '/login' } = {}) =>
  render(<MemoryRouter initialEntries={[route]}>{ui}</MemoryRouter>);

describe('GuestRoute', () => {
  it('renders children when user is not authenticated', () => {
    useAuthStore.setState({ isAuthenticated: false, isLoading: false, user: null });

    renderWithRouter(
      <GuestRoute>
        <div>Login Form</div>
      </GuestRoute>,
    );

    expect(screen.getByText('Login Form')).toBeInTheDocument();
  });

  it('redirects to / when user is authenticated', () => {
    useAuthStore.setState({
      isAuthenticated: true,
      isLoading: false,
      user: { id: 'user-1', name: 'Test', email: 'test@test.com', created_at: '', updated_at: '' },
    });

    const { container } = renderWithRouter(
      <GuestRoute>
        <div>Login Form</div>
      </GuestRoute>,
    );

    expect(screen.queryByText('Login Form')).not.toBeInTheDocument();
    // Navigate component renders nothing visible
    expect(container.innerHTML).toBe('');
  });

  it('shows loading state when auth is loading', () => {
    useAuthStore.setState({ isAuthenticated: false, isLoading: true, user: null });

    renderWithRouter(
      <GuestRoute>
        <div>Login Form</div>
      </GuestRoute>,
    );

    expect(screen.queryByText('Login Form')).not.toBeInTheDocument();
    expect(screen.getByText('Loading...')).toBeInTheDocument();
  });
});
