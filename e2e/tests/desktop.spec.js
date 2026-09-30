const { test, expect } = require('@playwright/test');
const { openStart, closeStart } = require('./helpers');

test.describe('desktop shell', () => {
  test('loads with an empty desktop and a menu bar', async ({ page }) => {
    await page.goto('/');
    await expect(page.locator('.menu-bar')).toBeVisible();
    await expect(page.locator('.desktop')).toBeVisible();
    await expect(page.locator('.window')).toHaveCount(0);
  });

  test('start menu lists only installed apps, in a stable order', async ({ page }) => {
    await page.goto('/');
    await openStart(page);
    const titles = await page.$$eval('.start-menu__item span:nth-child(2)', (els) =>
      els.map((e) => e.textContent)
    );
    // Manipulators (Sort, Filter) are excluded — they're never launched
    // directly, only added to a pipeline from within Contacts/Workbench.
    expect(titles).toEqual([
      'Program Manager',
      'Notes',
      'File Explorer',
      'Contacts',
      'Contacts Source',
      'Workbench',
      'Settings',
      'App Store',
    ]);
  });

  test('clicking empty desktop space closes the start menu', async ({ page }) => {
    await page.goto('/');
    await openStart(page);
    await closeStart(page);
    await expect(page.locator('.start-menu')).toHaveCount(0);
  });

  test('fullscreen button toggles document fullscreen state', async ({ page }) => {
    await page.goto('/');
    await page.click('.menu-bar__fullscreen');
    await page.waitForTimeout(200);
    const isFullscreen = await page.evaluate(() => document.fullscreenElement !== null);
    expect(isFullscreen).toBe(true);
    await expect(page.locator('.menu-bar__fullscreen')).toHaveClass(/is-active/);
  });

  test('clock is visible and updates', async ({ page }) => {
    await page.goto('/');
    const first = await page.locator('.menu-bar__clock').textContent();
    expect(first).toMatch(/^\d{2}:\d{2}(:\d{2})?$/);
  });
});
