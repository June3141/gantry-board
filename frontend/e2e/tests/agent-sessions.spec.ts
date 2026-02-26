import { expect, test } from '../fixtures';

test.describe('Agent Session UI', () => {
  test('shows agent controls in the task detail page', async ({
    authenticatedPage: page,
    apiHelper,
    testUser,
  }) => {
    const project = await apiHelper.createProject(testUser.cookie, `Agent Project ${Date.now()}`);
    const task = await apiHelper.createTask(testUser.cookie, project.id, 'Agent Test Task');

    await page.goto(`/projects/${project.id}/tasks/${task.id}`);

    // Agent controls should be visible in the Activity section
    await expect(page.getByLabel('Agent Type')).toBeVisible();
    await expect(page.getByPlaceholder(/enter prompt for the agent/i)).toBeVisible();
    await expect(page.getByRole('button', { name: 'Start' })).toBeVisible();
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
    const task = await apiHelper.createTask(testUser.cookie, project.id, 'Agent Options Task');

    await page.goto(`/projects/${project.id}/tasks/${task.id}`);

    // Agent type selector should have Claude Code and Gemini CLI options
    const agentSelect = page.locator('#timeline-agent-type');
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
    const task = await apiHelper.createTask(testUser.cookie, project.id, 'Start Disable Task');

    await page.goto(`/projects/${project.id}/tasks/${task.id}`);

    // Start button should be disabled when prompt is empty
    await expect(page.getByRole('button', { name: 'Start', exact: true })).toBeDisabled();

    // Fill in a prompt — start button should become enabled
    await page.getByPlaceholder(/enter prompt for the agent/i).fill('Test prompt');
    await expect(page.getByRole('button', { name: 'Start', exact: true })).toBeEnabled();
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
    const task = await apiHelper.createTask(testUser.cookie, project.id, 'Empty Activity Task');

    await page.goto(`/projects/${project.id}/tasks/${task.id}`);

    await expect(page.getByText(/no activity yet/i)).toBeVisible();
  });
});
