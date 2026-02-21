import { expect, test } from '../fixtures';

test.describe('Task CRUD', () => {
  test('creates a task via the kanban column button', async ({
    authenticatedPage: page,
    apiHelper,
    testUser,
  }) => {
    const project = await apiHelper.createProject(testUser.cookie, `Task Project ${Date.now()}`);

    await page.goto('/');
    await page.locator('#project-select').selectOption(project.id);

    // Click "Add Task" in the Backlog column
    await page.getByRole('button', { name: '+ Add Task' }).first().click();

    // Fill in the create task dialog
    const dialog = page.getByRole('dialog');
    await expect(dialog).toBeVisible();
    await dialog.getByLabel('Title').fill('E2E Test Task');
    await dialog.getByLabel('Description').fill('Created by E2E test');
    await dialog.getByRole('button', { name: 'Create' }).click();

    // Dialog should close and task card should appear
    await expect(dialog).not.toBeVisible();
    await expect(page.getByTestId('task-card').getByText('E2E Test Task')).toBeVisible();
  });

  test('opens task detail modal by clicking a task card', async ({
    authenticatedPage: page,
    apiHelper,
    testUser,
  }) => {
    const project = await apiHelper.createProject(testUser.cookie, `Detail Project ${Date.now()}`);
    await apiHelper.createTask(testUser.cookie, project.id, 'Detail Test Task', {
      description: 'Task with description',
    });

    await page.goto('/');
    await page.locator('#project-select').selectOption(project.id);

    // Click the task card
    await page.getByTestId('task-card').getByText('Detail Test Task').click();

    // Task detail modal should open
    const dialog = page.getByRole('dialog');
    await expect(dialog).toBeVisible();
    await expect(dialog.getByText('Detail Test Task')).toBeVisible();
    await expect(dialog.getByText('Task with description')).toBeVisible();
  });

  test('updates task status from the detail modal', async ({
    authenticatedPage: page,
    apiHelper,
    testUser,
  }) => {
    const project = await apiHelper.createProject(testUser.cookie, `Update Project ${Date.now()}`);
    await apiHelper.createTask(testUser.cookie, project.id, 'Status Change Task');

    await page.goto('/');
    await page.locator('#project-select').selectOption(project.id);

    // Open task detail
    await page.getByTestId('task-card').getByText('Status Change Task').click();

    const dialog = page.getByRole('dialog');
    await expect(dialog).toBeVisible();

    // Change status to "in_progress"
    await dialog.locator('#task-detail-status').selectOption('in_progress');

    // Close the modal
    await dialog.getByRole('button', { name: 'Close' }).click();

    // Task should now appear in the "In Progress" column
    await expect(
      page.locator('.flex-shrink-0', { hasText: 'In Progress' }).getByText('Status Change Task'),
    ).toBeVisible();
  });

  test('deletes a task from the detail modal', async ({
    authenticatedPage: page,
    apiHelper,
    testUser,
  }) => {
    const project = await apiHelper.createProject(testUser.cookie, `Delete Project ${Date.now()}`);
    await apiHelper.createTask(testUser.cookie, project.id, 'Task To Delete');

    await page.goto('/');
    await page.locator('#project-select').selectOption(project.id);

    // Open task detail and delete
    await page.getByTestId('task-card').getByText('Task To Delete').click();

    const dialog = page.getByRole('dialog');
    await dialog.getByRole('button', { name: 'Delete', exact: true }).click();
    await dialog.getByRole('button', { name: 'Confirm' }).click();

    // Modal should close and task should be gone
    await expect(dialog).not.toBeVisible();
    await expect(page.getByText('Task To Delete')).not.toBeVisible();
  });
});

test.describe('Task Comments', () => {
  test('adds a comment to a task', async ({ authenticatedPage: page, apiHelper, testUser }) => {
    const project = await apiHelper.createProject(testUser.cookie, `Comment Project ${Date.now()}`);
    await apiHelper.createTask(testUser.cookie, project.id, 'Commentable Task');

    await page.goto('/');
    await page.locator('#project-select').selectOption(project.id);

    // Open task detail
    await page.getByTestId('task-card').getByText('Commentable Task').click();
    const dialog = page.getByRole('dialog');
    await expect(dialog).toBeVisible();

    // Add a comment
    await dialog.getByPlaceholder('Add a comment...').fill('E2E test comment');
    await dialog.getByRole('button', { name: 'Post' }).click();

    // Comment should appear in the timeline
    await expect(dialog.getByText('E2E test comment')).toBeVisible();
  });

  test('displays existing comments', async ({ authenticatedPage: page, apiHelper, testUser }) => {
    const project = await apiHelper.createProject(
      testUser.cookie,
      `Pre-Comment Project ${Date.now()}`,
    );
    const task = await apiHelper.createTask(testUser.cookie, project.id, 'Pre-Commented Task');
    await apiHelper.createComment(testUser.cookie, task.id, 'Pre-existing comment');

    await page.goto('/');
    await page.locator('#project-select').selectOption(project.id);

    // Open task detail
    await page.getByTestId('task-card').getByText('Pre-Commented Task').click();
    const dialog = page.getByRole('dialog');

    // Pre-existing comment should be visible
    await expect(dialog.getByText('Pre-existing comment')).toBeVisible();
  });
});
