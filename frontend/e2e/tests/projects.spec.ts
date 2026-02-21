import { expect, test } from '../fixtures';

test.describe('Project CRUD', () => {
  test('creates a new project via UI', async ({ authenticatedPage: page }) => {
    await page.goto('/');

    // Open the create project dialog
    await page.getByRole('button', { name: 'New Project' }).click();

    // Fill in project details
    const dialog = page.getByRole('dialog');
    await expect(dialog).toBeVisible();
    await dialog.getByLabel('Name').fill('E2E Test Project');
    await dialog.getByLabel('Description').fill('Created by E2E test');
    await dialog.getByRole('button', { name: 'Create' }).click();

    // Dialog should close and project should appear in the selector
    await expect(dialog).not.toBeVisible();
    await expect(page.locator('#project-select')).toContainText('E2E Test Project');
  });

  test('selects a project and displays kanban board', async ({
    authenticatedPage: page,
    apiHelper,
    testUser,
  }) => {
    // Create a project via API
    const project = await apiHelper.createProject(testUser.cookie, `Board Project ${Date.now()}`);

    await page.goto('/');

    // Select the project
    await page.locator('#project-select').selectOption(project.id);

    // Kanban columns should be visible
    await expect(page.getByText('Backlog')).toBeVisible();
    await expect(page.getByText('To Do')).toBeVisible();
    await expect(page.getByText('In Progress')).toBeVisible();
    await expect(page.getByText('In Review')).toBeVisible();
    await expect(page.getByText('Done')).toBeVisible();
  });

  test('shows empty state when no project is selected', async ({ authenticatedPage: page }) => {
    await page.goto('/');
    await expect(page.getByText('Select a project to view its tasks')).toBeVisible();
  });

  test('opens project settings for selected project', async ({
    authenticatedPage: page,
    apiHelper,
    testUser,
  }) => {
    const project = await apiHelper.createProject(
      testUser.cookie,
      `Settings Project ${Date.now()}`,
    );

    await page.goto('/');
    await page.locator('#project-select').selectOption(project.id);

    // Settings button should appear
    await page.getByRole('button', { name: 'Settings' }).click();

    // Settings modal should be visible
    const dialog = page.getByRole('dialog');
    await expect(dialog).toBeVisible();
  });
});
