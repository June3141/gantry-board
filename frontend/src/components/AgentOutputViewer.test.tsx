import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { AgentOutputViewer } from './AgentOutputViewer';

describe('AgentOutputViewer', () => {
  it('renders empty state when no output', () => {
    render(<AgentOutputViewer lines={[]} />);
    expect(screen.getByText(/no output yet/i)).toBeInTheDocument();
  });

  it('renders output lines', () => {
    render(<AgentOutputViewer lines={['Hello', 'World']} />);
    expect(screen.getByText('Hello')).toBeInTheDocument();
    expect(screen.getByText('World')).toBeInTheDocument();
  });

  it('has scrollable container', () => {
    render(<AgentOutputViewer lines={['line1']} />);
    const container = screen.getByTestId('agent-output-container');
    expect(container).toHaveClass('overflow-y-auto');
  });

  it('renders with terminal styling', () => {
    render(<AgentOutputViewer lines={['test']} />);
    const container = screen.getByTestId('agent-output-container');
    expect(container).toHaveClass('bg-gray-900');
    expect(container).toHaveClass('font-mono');
  });
});
