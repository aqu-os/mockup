const { test, expect } = require('@playwright/test');
const { launchApp, installApp, uninstallApp, openStart, connectContactsSource } = require('./helpers');

test.describe('App Store', () => {
  test('lists every app with correct install state', async ({ page }) => {
    await page.goto('/');
    await launchApp(page, 'App Store');
    await page.waitForSelector('.store');

    const rows = await page.$$eval('.store-item', (els) =>
      els.map((el) => ({
        title: el.querySelector('.store-item__title')?.textContent,
        badge: el.querySelector('.store-item__badge')?.textContent ?? null,
        install: !!el.querySelector('.store-item__btn--primary'),
        uninstall: !!el.querySelector('.store-item__btn--danger'),
      }))
    );

    // The App Store manages the whole catalog, including the source and
    // manipulator apps behind Contacts — it doesn't filter by role the
    // way the start menu does (see desktop.spec.js).
    expect(rows).toEqual([
      { title: 'Program Manager', badge: 'System', install: false, uninstall: false },
      { title: 'Notes', badge: null, install: false, uninstall: true },
      { title: 'File Explorer', badge: null, install: false, uninstall: true },
      { title: 'Contacts', badge: null, install: false, uninstall: true },
      { title: 'Contacts Source', badge: null, install: false, uninstall: true },
      { title: 'Sort', badge: null, install: false, uninstall: true },
      { title: 'Filter', badge: null, install: false, uninstall: true },
      { title: 'Workbench', badge: 'System', install: false, uninstall: false },
      { title: 'Settings', badge: 'System', install: false, uninstall: false },
      { title: 'App Store', badge: 'System', install: false, uninstall: false },
      { title: 'Calculator', badge: null, install: true, uninstall: false },
    ]);
  });

  test('installing an app makes it launchable from the start menu', async ({ page }) => {
    await page.goto('/');
    await installApp(page, 'Calculator');
    await openStart(page);
    await expect(page.locator('.start-menu__item:has-text("Calculator")')).toHaveCount(1);
  });

  test('uninstalling closes running instances and removes it from the start menu', async ({ page }) => {
    await page.goto('/');
    await launchApp(page, 'Notes');
    await expect(page.locator('.window')).toHaveCount(1);

    await uninstallApp(page, 'Notes');
    // Notes window closed; only the App Store window (used to uninstall) remains.
    await expect(page.locator('.window')).toHaveCount(1);
    await expect(page.locator('.window__title')).toHaveText('App Store');

    await openStart(page);
    await expect(page.locator('.start-menu__item:has-text("Notes")')).toHaveCount(0);
  });

  test('uninstalling Contacts Source without removing data leaves the running pipeline untouched', async ({
    page,
  }) => {
    await page.goto('/');
    await launchApp(page, 'Contacts');
    await connectContactsSource(page);
    await page.fill('.contacts__input[placeholder="Name"]', 'Keep Me');
    await page.fill('.contacts__input[placeholder="Email"]', 'keep@aquos.dev');
    await page.click('.contacts__add-btn');

    // Contacts Source has no window of its own here (never opened
    // directly), so uninstalling only removes it from the catalog —
    // the headless process backing the already-open Contacts window
    // keeps running, same as a real OS wouldn't kill a process just for
    // being uninstalled while still running.
    await uninstallApp(page, 'Contacts Source', { removeData: false });
    await installApp(page, 'Contacts Source');
    await expect(page.locator('.contact-item__name')).toContainText(['Keep Me']);
  });

  test('uninstalling Contacts Source with "remove personalized data" tears down its pipeline', async ({
    page,
  }) => {
    await page.goto('/');
    await launchApp(page, 'Contacts');
    await connectContactsSource(page);
    await expect(page.locator('.contact-item__name')).toHaveCount(3); // seed data present

    await uninstallApp(page, 'Contacts Source', { removeData: true });
    // The pipeline the still-open Contacts window depended on is gone —
    // it falls back to asking for a data source again, same as a fresh
    // launch.
    await expect(page.locator('.contacts-sink__builder')).toBeVisible();
    await expect(page.locator('.contact-item__name')).toHaveCount(0);

    await installApp(page, 'Contacts Source');
    // installApp leaves the App Store window focused/on top of Contacts
    // (both floating in workspace 1) — bring Contacts back to front
    // before clicking into it.
    await launchApp(page, 'Contacts');
    await connectContactsSource(page);
    await expect(page.locator('.contact-item__name')).toHaveCount(3); // fresh seed data
  });

  test('system apps cannot be uninstalled', async ({ page }) => {
    await page.goto('/');
    await launchApp(page, 'App Store');
    const settingsRow = page.locator('.store-item', { hasText: 'Settings' });
    await expect(settingsRow.locator('.store-item__btn--danger')).toHaveCount(0);
    await expect(settingsRow.locator('.store-item__badge')).toHaveText('System');
  });
});
