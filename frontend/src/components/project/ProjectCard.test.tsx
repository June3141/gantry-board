import { render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { describe, expect, it } from 'vitest';
import { ProjectCard } from './ProjectCard';

const renderWithRouter = (ui: React.ReactElement) => render(<MemoryRouter>{ui}</MemoryRouter>);

describe('ProjectCard', () => {
  it('renders project name', () => {
    renderWithRouter(
      <ProjectCard project={{ id: 'p1', name: 'My Project', created_at: '', updated_at: '' }} />,
    );

    expect(screen.getByText('My Project')).toBeInTheDocument();
  });

  it('renders description when provided', () => {
    renderWithRouter(
      <ProjectCard
        project={{
          id: 'p1',
          name: 'My Project',
          description: 'A great project',
          created_at: '',
          updated_at: '',
        }}
      />,
    );

    expect(screen.getByText('A great project')).toBeInTheDocument();
  });

  it('links to project board', () => {
    renderWithRouter(
      <ProjectCard project={{ id: 'p1', name: 'My Project', created_at: '', updated_at: '' }} />,
    );

    const link = screen.getByRole('link');
    expect(link).toHaveAttribute('href', '/projects/p1');
  });
});
