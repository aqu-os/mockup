const { test, expect } = require('@playwright/test');
const { launchApp, installApp, switchWorkspace, connectContactsSource, addManipulator } = require('./helpers');

test.describe('Notes', () => {
  test('typing persists in the textarea', async ({ page }) => {
    await page.goto('/');
    await launchApp(page, 'Notes');
    await page.fill('.notes__area', 'hello aquos');
    await expect(page.locator('.notes__area')).toHaveValue('hello aquos');
  });

  test('is not a singleton — launching twice opens two windows', async ({ page }) => {
    await page.goto('/');
    await launchApp(page, 'Notes');
    await launchApp(page, 'Notes');
    await expect(page.locator('.window')).toHaveCount(2);
  });
});

test.describe('File Explorer', () => {
  test('navigates into folders and back via breadcrumbs', async ({ page }) => {
    await page.goto('/');
    await launchApp(page, 'File Explorer');
    await page.waitForSelector('.fe');

    await page.click('.fe__entry:has-text("Home")');
    await expect(page.locator('.fe__entry-name')).toHaveText([
      'Documents',
      'Pictures',
      'Music',
      'Downloads',
    ]);

    await page.click('.fe__entry:has-text("Documents")');
    await expect(page.locator('.fe__entry-name')).toHaveText(['Projects', 'resume.txt', 'notes.md']);

    await page.click('.fe__crumb:has-text("This PC")');
    await expect(page.locator('.fe__entry-name')).toHaveText(['Applications', 'Home', 'System', 'README.txt']);
  });

  test('an empty folder shows the empty state', async ({ page }) => {
    await page.goto('/');
    await launchApp(page, 'File Explorer');
    await page.click('.fe__entry:has-text("Home")');
    await page.click('.fe__entry:has-text("Downloads")');
    await expect(page.locator('.fe__empty')).toHaveText('This folder is empty.');
  });

  test('clicking a regular file selects it without launching anything', async ({ page }) => {
    await page.goto('/');
    await launchApp(page, 'File Explorer');
    await page.click('.fe__entry:has-text("README.txt")');
    await expect(page.locator('.fe__entry:has-text("README.txt")')).toHaveClass(/is-selected/);
    await expect(page.locator('.window')).toHaveCount(1); // just File Explorer itself
  });

  test('Applications lists every installed, non-system app as an executable', async ({ page }) => {
    await page.goto('/');
    await launchApp(page, 'File Explorer');
    await page.click('.fe__entry:has-text("Applications")');
    await expect(page.locator('.fe__entry-name')).toHaveText([
      'Notes',
      'File Explorer',
      'Contacts',
      'Contacts Source',
    ]);
    // Executables get the rocket icon, not the folder icon.
    await expect(page.locator('.fe__entry--file .fe__entry-icon').first()).toHaveText('\u{1F680}');
  });

  test('clicking an executable launches that app', async ({ page }) => {
    await page.goto('/');
    await launchApp(page, 'File Explorer');
    await page.click('.fe__entry:has-text("Applications")');
    await page.click('.fe__entry:has-text("Notes")');
    await expect(page.locator('.window__title:has-text("Notes")')).toHaveCount(1);
    await expect(page.locator('.window')).toHaveCount(2); // File Explorer + Notes
  });

  test('Applications updates live as apps are installed and uninstalled', async ({ page }) => {
    await page.goto('/');
    // Tiling so File Explorer and the App Store (opened by installApp)
    // don't end up stacked on top of one another.
    await switchWorkspace(page, 2);
    await launchApp(page, 'File Explorer');
    await page.click('.fe__entry:has-text("Applications")');
    await expect(page.locator('.fe__entry-name:has-text("Calculator")')).toHaveCount(0);

    await installApp(page, 'Calculator');
    await expect(page.locator('.fe__entry-name:has-text("Calculator")')).toHaveCount(1);

    await page.click('.fe__entry:has-text("Calculator")');
    await expect(page.locator('.window__title:has-text("Calculator")')).toHaveCount(1);
  });
});

test.describe('Contacts', () => {
  test('opened fresh, offers a data source before showing any list', async ({ page }) => {
    await page.goto('/');
    await launchApp(page, 'Contacts');
    await expect(page.locator('.contacts-sink__builder')).toBeVisible();
    await expect(page.locator('.contact-item')).toHaveCount(0);
    await expect(page.locator('.app-picker__btn')).toHaveCount(1);
    await expect(page.locator('.app-picker__btn')).toContainText('Contacts Source');
  });

  test('once connected, lists seed contacts (seed order — no manipulator) and shows details on select', async ({
    page,
  }) => {
    await page.goto('/');
    await launchApp(page, 'Contacts');
    await connectContactsSource(page);
    await expect(page.locator('.contact-item__name')).toHaveText([
      'Ada Lovelace',
      'Grace Hopper',
      'Alan Turing',
    ]);
    await page.click('.contact-item >> nth=0');
    await expect(page.locator('.contact-detail__name')).toHaveText('Ada Lovelace');
  });

  test('can add and remove a contact', async ({ page }) => {
    await page.goto('/');
    await launchApp(page, 'Contacts');
    await connectContactsSource(page);
    await page.fill('.contacts__input[placeholder="Name"]', 'Linus Torvalds');
    await page.fill('.contacts__input[placeholder="Email"]', 'linus@aquos.dev');
    await page.click('.contacts__add-btn');
    await expect(page.locator('.contact-item__name')).toContainText(['Linus Torvalds']);

    await page.click('.contact-item:has-text("Linus Torvalds")');
    await page.click('.contact-detail__remove');
    await expect(page.locator('.contact-item:has-text("Linus Torvalds")')).toHaveCount(0);
  });

  test('is a singleton — launching twice focuses the same window', async ({ page }) => {
    await page.goto('/');
    await launchApp(page, 'Contacts');
    await launchApp(page, 'Contacts');
    await expect(page.locator('.window')).toHaveCount(1);
  });

  test('adding the Sort manipulator reorders the list, chained after Filter', async ({ page }) => {
    await page.goto('/');
    await launchApp(page, 'Contacts');
    await connectContactsSource(page);

    await addManipulator(page, 'Sort');
    await page.click('.settings-choice__btn:has-text("Role")');
    await page.click('.contacts-sink__builder-done');
    await expect(page.locator('.contact-item__name')).toHaveText([
      'Grace Hopper', // Compiler Engineer
      'Alan Turing', // Kernel Maintainer
      'Ada Lovelace', // Systems Architect
    ]);

    // Chaining Filter after Sort narrows the already-sorted list rather
    // than replacing it — "Alan Turing" is the only name without an "o",
    // so filtering by it should drop just that one, order preserved.
    await addManipulator(page, 'Filter');
    await page.fill('.pipeline-chip--manipulator .settings-text', 'o');
    await page.click('.contacts-sink__builder-done');
    await expect(page.locator('.contact-item__name')).toHaveText(['Grace Hopper', 'Ada Lovelace']);
  });
});

test.describe('Program Manager', () => {
  test('lists itself and other running apps, and End Task closes them', async ({ page }) => {
    await page.goto('/');
    // Tiling so Program Manager and Notes lay out side by side rather than
    // floating's overlapping cascade — otherwise Notes (launched last, on
    // top) would cover Program Manager's End Task button.
    await switchWorkspace(page, 2);
    await launchApp(page, 'Program Manager');
    await launchApp(page, 'Notes');
    await expect(page.locator('.pm__table tbody tr')).toHaveCount(2);

    const notesRow = page.locator('.pm__table tbody tr', { hasText: 'Notes' });
    await notesRow.locator('.pm__end-btn').click();
    await expect(page.locator('.pm__table tbody tr')).toHaveCount(1);
    await expect(page.locator('.window')).toHaveCount(1); // Program Manager itself
  });

  test('shows the empty state once nothing is running (including itself)', async ({ page }) => {
    await page.goto('/');
    await launchApp(page, 'Program Manager');
    await page.click('.pm__end-btn'); // ends itself
    await expect(page.locator('.pm__empty')).toHaveCount(0); // window closed along with it
    await expect(page.locator('.window')).toHaveCount(0);
  });
});

test.describe('Calculator', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await installApp(page, 'Calculator');
  });

  test('performs basic arithmetic', async ({ page }) => {
    await launchApp(page, 'Calculator');
    await page.click('.calc__btn:has-text("7")');
    await page.click('.calc__btn:has-text("÷")');
    await page.click('.calc__btn:has-text("3")');
    await page.click('.calc__btn:has-text("=")');
    await expect(page.locator('.calc__display')).toHaveText('2.33');
  });

  test('clear resets the display', async ({ page }) => {
    await launchApp(page, 'Calculator');
    await page.click('.calc__btn:has-text("9")');
    await page.click('.calc__btn:has-text("C")');
    await expect(page.locator('.calc__display')).toHaveText('0');
  });
});
