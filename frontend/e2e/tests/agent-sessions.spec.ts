import { expect, test } from '../fixtures';

test.describe('Agent Session UI', () => {
  test('shows agent controls in the task detail modal', async ({
    authenticatedPage: page,
    apiHelper,
    testUser,
  }) => {
    const project = await apiHelper.createProject(testUser.cookie, `Agent Project ${Date.now()}`);
    await apiHelper.createTask(testUser.cookie, project.id, 'Agent Test Task');

    await page.goto('/');
    await page.locator('#project-select').selectOption(project.id);

    // Open task detail
    await page.getByTestId('task-card').getByText('Agent Test Task').click();
    const dialog = page.getByRole('dialog');
    await expect(dialog).toBeVisible();

    // Agent controls should be visible in the Activity section
    await expect(dialog.getByLabel('Agent Type')).toBeVisible();
    await expect(dialog.getByPlaceholder('Enter prompt for the agent...')).toBeVisible();
    await expect(dialog.getByRole('button', { name: 'Start' })).toBeVisible();
  });

  test('agent type selector has expected options', async ({
    authenticatedPage: page,
    apiHelper,
    testUser,
  }) => {
    const project = await apiHelper.createProject(
      testUser.cookie,
      `Agent Options Project ${Date.now()}`,
    );
    await apiHelper.createTask(testUser.cookie, project.id, 'Agent Options Task');

    await page.goto('/');
    await page.locator('#project-select').selectOption(project.id);

    // Open task detail
    await page.getByTestId('task-card').getByText('Agent Options Task').click();
    const dialog = page.getByRole('dialog');

    // Agent type selector should have Claude Code and Gemini CLI options
    const agentSelect = dialog.locator('#timeline-agent-type');
    await expect(agentSelect.locator('option', { hasText: 'Claude Code' })).toBeAttached();
    await expect(agentSelect.locator('option', { hasText: 'Gemini CLI' })).toBeAttached();
  });

  test('start button is disabled when prompt is empty', async ({
    authenticatedPage: page,
    apiHelper,
    testUser,
  }) => {
    const project = await apiHelper.createProject(
      testUser.cookie,
      `Start Disable Project ${Date.now()}`,
    );
    await apiHelper.createTask(testUser.cookie, project.id, 'Start Disable Task');

    await page.goto('/');
    await page.locator('#project-select').selectOption(project.id);

    await page.getByTestId('task-card').getByText('Start Disable Task').click();
    const dialog = page.getByRole('dialog');

    // Start button should be disabled when prompt is empty
    await expect(dialog.getByRole('button', { name: 'Start' })).toBeDisabled();

    // Fill in a prompt — start button should become enabled
    await dialog.getByPlaceholder('Enter prompt for the agent...').fill('Test prompt');
    await expect(dialog.getByRole('button', { name: 'Start' })).toBeEnabled();
  });

  test('shows "No activity yet" when task has no sessions or comments', async ({
    authenticatedPage: page,
    apiHelper,
    testUser,
  }) => {
    const project = await apiHelper.createProject(
      testUser.cookie,
      `Empty Activity Project ${Date.now()}`,
    );
    await apiHelper.createTask(testUser.cookie, project.id, 'Empty Activity Task');

    await page.goto('/');
    await page.locator('#project-select').selectOption(project.id);

    await page.getByTestId('task-card').getByText('Empty Activity Task').click();
    const dialog = page.getByRole('dialog');

    await expect(dialog.getByText('No activity yet.')).toBeVisible();
  });
});
