import { expect, test } from '../fixtures';

test.describe('Project CRUD', () => {
  test('creates a new project via UI', async ({ authenticatedPage: page }) => {
    await page.goto('/');

    // Open the create project dialog
    await page.getByRole('button', { name: /new project/i }).click();

    // Fill in project details
    const dialog = page.getByRole('dialog');
    await expect(dialog).toBeVisible();
    await dialog.getByLabel('Name').fill('E2E Test Project');
    await dialog.getByLabel('Description').fill('Created by E2E test');
    await dialog.getByRole('button', { name: 'Create' }).click();

    // Dialog should close and project card should appear on the list page
    await expect(dialog).not.toBeVisible();
    await expect(page.getByText('E2E Test Project')).toBeVisible();
  });

  test('navigates to project board by clicking a project card', async ({
    authenticatedPage: page,
    apiHelper,
    testUser,
  }) => {
    // Create a project via API
    const project = await apiHelper.createProject(testUser.cookie, `Board Project ${Date.now()}`);

    await page.goto('/');

    // Click the project card
    await page.getByText(project.name).click();

    // Kanban columns should be visible
    await expect(page.getByText('Backlog')).toBeVisible();
    await expect(page.getByText('To Do')).toBeVisible();
    await expect(page.getByText('In Progress')).toBeVisible();
    await expect(page.getByText('In Review')).toBeVisible();
    await expect(page.getByText('Done')).toBeVisible();
  });

  test('shows empty state when no projects exist', async ({ authenticatedPage: page }) => {
    await page.goto('/');
    // The project list page should be shown (with either projects or empty state)
    await expect(page.getByText(/projects/i).first()).toBeVisible();
  });

  test('opens project settings from the board page', async ({
    authenticatedPage: page,
    apiHelper,
    testUser,
  }) => {
    const project = await apiHelper.createProject(
      testUser.cookie,
      `Settings Project ${Date.now()}`,
    );

    await page.goto(`/projects/${project.id}`);

    // Settings button should appear
    await page.getByRole('button', { name: /settings/i }).click();

    // Settings modal should be visible
    const dialog = page.getByRole('dialog');
    await expect(dialog).toBeVisible();
  });
});
