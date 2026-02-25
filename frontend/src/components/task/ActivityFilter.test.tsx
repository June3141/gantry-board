import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { ActivityFilter, type ActivityFilterValue } from './ActivityFilter';

describe('ActivityFilter', () => {
  it('renders two toggle buttons', () => {
    render(<ActivityFilter value="all" onChange={vi.fn()} />);

    expect(screen.getByRole('button', { name: /all activity/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /comments only/i })).toBeInTheDocument();
  });

  it('highlights "All Activity" button when value is "all"', () => {
    render(<ActivityFilter value="all" onChange={vi.fn()} />);

    const allBtn = screen.getByRole('button', { name: /all activity/i });
    const commentsBtn = screen.getByRole('button', { name: /comments only/i });

    expect(allBtn).toHaveAttribute('aria-pressed', 'true');
    expect(commentsBtn).toHaveAttribute('aria-pressed', 'false');
  });

  it('highlights "Comments Only" button when value is "comments"', () => {
    render(<ActivityFilter value="comments" onChange={vi.fn()} />);

    const allBtn = screen.getByRole('button', { name: /all activity/i });
    const commentsBtn = screen.getByRole('button', { name: /comments only/i });

    expect(allBtn).toHaveAttribute('aria-pressed', 'false');
    expect(commentsBtn).toHaveAttribute('aria-pressed', 'true');
  });

  it('calls onChange with "comments" when "Comments Only" is clicked', async () => {
    const onChange = vi.fn();
    const user = userEvent.setup();
    render(<ActivityFilter value="all" onChange={onChange} />);

    await user.click(screen.getByRole('button', { name: /comments only/i }));

    expect(onChange).toHaveBeenCalledWith('comments');
  });

  it('calls onChange with "all" when "All Activity" is clicked', async () => {
    const onChange = vi.fn();
    const user = userEvent.setup();
    render(<ActivityFilter value="comments" onChange={onChange} />);

    await user.click(screen.getByRole('button', { name: /all activity/i }));

    expect(onChange).toHaveBeenCalledWith('all');
  });

  it('does not call onChange when already-selected button is clicked', async () => {
    const onChange = vi.fn();
    const user = userEvent.setup();
    render(<ActivityFilter value="all" onChange={onChange} />);

    await user.click(screen.getByRole('button', { name: /all activity/i }));

    expect(onChange).not.toHaveBeenCalled();
  });
});
