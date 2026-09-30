const { test, expect } = require('@playwright/test');
const { launchApp, switchWorkspace, switchLayout, visibleWindowCount } = require('./helpers');

test.describe('workspaces', () => {
  test('each workspace defaults to a different layout', async ({ page }) => {
    await page.goto('/');
    await switchWorkspace(page, 1);
    await expect(page.locator('.layout-pill[title="Floating"]')).toHaveClass(/is-active/);
    await switchWorkspace(page, 2);
    await expect(page.locator('.layout-pill[title="Tiling"]')).toHaveClass(/is-active/);
    await switchWorkspace(page, 3);
    await expect(page.locator('.layout-pill[title="Infinite scroll"]')).toHaveClass(/is-active/);
  });

  test('windows and layout persist when switching away and back', async ({ page }) => {
    await page.goto('/');
    await switchWorkspace(page, 1);
    await launchApp(page, 'Notes');
    await launchApp(page, 'Notes');
    expect(await visibleWindowCount(page)).toBe(2);

    await switchWorkspace(page, 2);
    expect(await visibleWindowCount(page)).toBe(0);

    await switchWorkspace(page, 1);
    expect(await visibleWindowCount(page)).toBe(2);
    await expect(page.locator('.layout-pill[title="Floating"]')).toHaveClass(/is-active/);
  });

  test('taskbar only shows the active workspace\'s windows', async ({ page }) => {
    await page.goto('/');
    await switchWorkspace(page, 1);
    await launchApp(page, 'Notes');
    await expect(page.locator('.taskbar-item')).toHaveCount(1);

    await switchWorkspace(page, 2);
    await expect(page.locator('.taskbar-item')).toHaveCount(0);
    await launchApp(page, 'Notes');
    await expect(page.locator('.taskbar-item')).toHaveCount(1);

    await switchWorkspace(page, 1);
    await expect(page.locator('.taskbar-item')).toHaveCount(1);
  });

  test('layout can be changed per workspace at runtime', async ({ page }) => {
    await page.goto('/');
    await switchWorkspace(page, 1);
    await launchApp(page, 'Notes');
    await launchApp(page, 'Notes');

    await switchLayout(page, 'Tiling');
    await expect(page.locator('.desktop__windows')).toHaveClass(/desktop__windows--tiling/);
    const grow = await page.$$eval('.window', (els) => els.map((el) => el.style.flexGrow));
    expect(grow).toEqual(['0.5', '0.5']);
  });
});
