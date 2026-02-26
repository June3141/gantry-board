import { expect, test } from '../fixtures';

test.describe('Task CRUD', () => {
  test('creates a task via the kanban column button', async ({
    authenticatedPage: page,
    apiHelper,
    testUser,
  }) => {
    const project = await apiHelper.createProject(testUser.cookie, `Task Project ${Date.now()}`);

    await page.goto(`/projects/${project.id}`);

    // Click "Add Task" in the Backlog column
    await page.getByRole('button', { name: /add task/i }).first().click();

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

  test('opens task detail page by clicking a task card', async ({
    authenticatedPage: page,
    apiHelper,
    testUser,
  }) => {
    const project = await apiHelper.createProject(testUser.cookie, `Detail Project ${Date.now()}`);
    await apiHelper.createTask(testUser.cookie, project.id, 'Detail Test Task', {
      description: 'Task with description',
    });

    await page.goto(`/projects/${project.id}`);

    // Click the task card
    await page.getByTestId('task-card').getByText('Detail Test Task').click();

    // Task detail page should show task info
    await expect(page.getByText('Detail Test Task')).toBeVisible();
    await expect(page.getByText('Task with description')).toBeVisible();
  });

  test('updates task status from the detail page', async ({
    authenticatedPage: page,
    apiHelper,
    testUser,
  }) => {
    const project = await apiHelper.createProject(testUser.cookie, `Update Project ${Date.now()}`);
    const task = await apiHelper.createTask(testUser.cookie, project.id, 'Status Change Task');

    await page.goto(`/projects/${project.id}/tasks/${task.id}`);

    // Change status to "in_progress"
    await page.locator('#task-status').selectOption('in_progress');

    // Navigate back to the board
    await page.getByRole('link', { name: /back/i }).click();

    // Task should now appear in the "In Progress" column
    await expect(
      page.locator('.flex-shrink-0', { hasText: 'In Progress' }).getByText('Status Change Task'),
    ).toBeVisible();
  });

  test('deletes a task from the detail page', async ({
    authenticatedPage: page,
    apiHelper,
    testUser,
  }) => {
    const project = await apiHelper.createProject(testUser.cookie, `Delete Project ${Date.now()}`);
    const task = await apiHelper.createTask(testUser.cookie, project.id, 'Task To Delete');

    await page.goto(`/projects/${project.id}/tasks/${task.id}`);

    // Delete the task
    await page.getByRole('button', { name: /delete task/i }).click();
    await page.getByRole('button', { name: /confirm/i }).click();

    // Should navigate back to board and task should be gone
    await expect(page.getByText('Task To Delete')).not.toBeVisible();
  });
});

test.describe('Task Comments', () => {
  test('adds a comment to a task', async ({ authenticatedPage: page, apiHelper, testUser }) => {
    const project = await apiHelper.createProject(testUser.cookie, `Comment Project ${Date.now()}`);
    const task = await apiHelper.createTask(testUser.cookie, project.id, 'Commentable Task');

    await page.goto(`/projects/${project.id}/tasks/${task.id}`);

    // Add a comment
    await page.getByPlaceholder(/add a comment/i).fill('E2E test comment');
    await page.getByRole('button', { name: /post/i }).click();

    // Comment should appear in the timeline
    await expect(page.getByText('E2E test comment')).toBeVisible();
  });

  test('displays existing comments', async ({ authenticatedPage: page, apiHelper, testUser }) => {
    const project = await apiHelper.createProject(
      testUser.cookie,
      `Pre-Comment Project ${Date.now()}`,
    );
    const task = await apiHelper.createTask(testUser.cookie, project.id, 'Pre-Commented Task');
    await apiHelper.createComment(testUser.cookie, task.id, 'Pre-existing comment');

    await page.goto(`/projects/${project.id}/tasks/${task.id}`);

    // Pre-existing comment should be visible
    await expect(page.getByText('Pre-existing comment')).toBeVisible();
  });
});
