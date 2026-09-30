const { test, expect } = require('@playwright/test');
const {
  launchApp,
  installApp,
  openSettingsSection,
  switchWorkspace,
  clickWindowAction,
} = require('./helpers');

test.describe('Settings', () => {
  test('sidebar lists OS categories and installed apps with settings', async ({ page }) => {
    await page.goto('/');
    await launchApp(page, 'Settings');
    // Use the last child text node rather than full textContent: app
    // entries prefix an icon <span>, OS category entries don't.
    const items = await page.$$eval('.settings__nav-item', (els) =>
      els.map((e) => e.lastChild.textContent.trim())
    );
    // Program Manager, Settings and App Store have no settings, so they're
    // absent from the Apps section — as are Contacts and the apps behind
    // it (Contacts Source, Sort, Filter, Workbench): Contacts' old
    // sort-by setting is now the Sort manipulator's pipeline-chip
    // parameter instead (see apps.spec.js).
    expect(items).toEqual(['Desktop', 'Window Managers', 'About', 'Notes', 'File Explorer']);
  });

  test('an uninstalled app disappears from the sidebar live', async ({ page }) => {
    await page.goto('/');
    await installApp(page, 'Calculator');
    await openSettingsSection(page, 'Desktop');
    await expect(page.locator('.settings__nav-item:has-text("Calculator")')).toHaveCount(1);
  });

  test('Desktop wallpaper choice repaints the desktop', async ({ page }) => {
    await page.goto('/');
    await openSettingsSection(page, 'Desktop');
    await page.click('.settings-choice__btn:has-text("Solid Gradient")');
    await expect(page.locator('.desktop')).toHaveClass(/desktop--solid/);

    await page.click('.settings-choice__btn:has-text("Mountains (Warm)")');
    await expect(page.locator('.desktop')).toHaveClass(/desktop--warm/);
  });

  test('About: hiding seconds changes the clock format', async ({ page }) => {
    await page.goto('/');
    await openSettingsSection(page, 'About');
    await page.click('.settings-toggle');
    await page.waitForTimeout(1100);
    await expect(page.locator('.menu-bar__clock')).toHaveText(/^\d{2}:\d{2}$/);
  });

  test('Window Managers: tiling step change affects actual resize amount', async ({ page }) => {
    await page.goto('/');
    await openSettingsSection(page, 'Window Managers');
    const tileSlider = page.locator('.settings-number input[type="range"]').first();
    await tileSlider.fill('0.2');

    await switchWorkspace(page, 2);
    await launchApp(page, 'Notes');
    await launchApp(page, 'Notes');
    await clickWindowAction(page, 1, 'Widen');
    const grow = await page.$$eval('.window', (els) => els.map((el) => parseFloat(el.style.flexGrow)));
    expect(grow[0]).toBeCloseTo(0.7, 1); // 0.5 + 0.2
  });

  test('Notes: font size choice changes the textarea font-size', async ({ page }) => {
    await page.goto('/');
    await launchApp(page, 'Notes');
    await openSettingsSection(page, 'Notes');
    await page.click('.settings-choice__btn:has-text("Large")');
    await expect(page.locator('.notes__area')).toHaveCSS('font-size', '16px');
  });

  test('File Explorer: hiding sizes removes the size labels', async ({ page }) => {
    await page.goto('/');
    await launchApp(page, 'File Explorer');
    await page.click('.fe__entry:has-text("Home")');
    await page.click('.fe__entry:has-text("Documents")');
    await expect(page.locator('.fe__entry-size')).toHaveCount(2);

    await openSettingsSection(page, 'File Explorer');
    await page.click('.settings-toggle');
    await expect(page.locator('.fe__entry-size')).toHaveCount(0);
  });

  test('Calculator: precision setting changes displayed decimal places', async ({ page }) => {
    await page.goto('/');
    await installApp(page, 'Calculator');
    await launchApp(page, 'Calculator');
    await page.click('.calc__btn:has-text("7")');
    await page.click('.calc__btn:has-text("÷")');
    await page.click('.calc__btn:has-text("3")');
    await page.click('.calc__btn:has-text("=")');
    await expect(page.locator('.calc__display')).toHaveText('2.33');

    await openSettingsSection(page, 'Calculator');
    const precisionSlider = page.locator('.settings-number input[type="range"]');
    await precisionSlider.fill('4');

    await launchApp(page, 'Calculator');
    await page.click('.calc__btn:has-text("7")');
    await page.click('.calc__btn:has-text("÷")');
    await page.click('.calc__btn:has-text("3")');
    await page.click('.calc__btn:has-text("=")');
    await expect(page.locator('.calc__display')).toHaveText('2.3333');
  });
});
