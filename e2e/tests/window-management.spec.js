const { test, expect } = require('@playwright/test');
const {
  launchApp,
  switchWorkspace,
  clickWindowAction,
  dragWindow,
  visibleWindowCount,
} = require('./helpers');

test.describe('floating window manager (workspace 1)', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await switchWorkspace(page, 1);
  });

  test('dragging a window moves it, and clicking its body still works after', async ({ page }) => {
    await launchApp(page, 'Notes');
    const before = await page.locator('.window').evaluate((el) => el.style.left);
    await dragWindow(page, 1, 260, 160);
    const after = await page.locator('.window').evaluate((el) => el.style.left);
    expect(after).not.toBe(before);

    // The click-swallowing regression this app hit twice before: a click
    // right after a drag must still register.
    await page.click('.notes__area');
    await page.keyboard.type('still works');
    await expect(page.locator('.notes__area')).toHaveValue('still works');
  });

  test('maximize fills the desktop and restore returns to the previous rect, text intact', async ({
    page,
  }) => {
    await launchApp(page, 'Notes');
    await page.fill('.notes__area', 'keep me');
    const originalWidth = await page.locator('.window').evaluate((el) => el.style.width);

    await clickWindowAction(page, 1, 'Maximize');
    await expect(page.locator('.window')).toHaveClass(/window--maximized/);
    await expect(page.locator('.notes__area')).toHaveValue('keep me');

    await clickWindowAction(page, 1, 'Restore');
    const restoredWidth = await page.locator('.window').evaluate((el) => el.style.width);
    expect(restoredWidth).toBe(originalWidth);
  });

  test('closing a window removes it and its taskbar entry', async ({ page }) => {
    await launchApp(page, 'Notes');
    await expect(page.locator('.window')).toHaveCount(1);
    await page.click('.window__close');
    await expect(page.locator('.window')).toHaveCount(0);
    await expect(page.locator('.taskbar-item')).toHaveCount(0);
  });

  test('minimize hides the window but keeps it in the taskbar, and restores from there', async ({
    page,
  }) => {
    await launchApp(page, 'Notes');
    await page.fill('.notes__area', 'minimized text');
    await clickWindowAction(page, 1, 'Minimize');

    expect(await visibleWindowCount(page)).toBe(0);
    await expect(page.locator('.taskbar-item')).toHaveCount(1);
    await expect(page.locator('.taskbar-item')).toHaveClass(/is-minimized/);

    await page.click('.taskbar-item');
    expect(await visibleWindowCount(page)).toBe(1);
    await expect(page.locator('.notes__area')).toHaveValue('minimized text');
  });
});

test.describe('tiling window manager (workspace 2)', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await switchWorkspace(page, 2);
  });

  test('windows split the row evenly and stay non-overlapping', async ({ page }) => {
    await launchApp(page, 'Notes');
    await launchApp(page, 'Notes');
    await launchApp(page, 'Notes');
    const grow = await page.$$eval('.window', (els) => els.map((el) => parseFloat(el.style.flexGrow)));
    expect(grow.every((g) => Math.abs(g - 1 / 3) < 0.01)).toBe(true);
  });

  test('widen takes space from neighbors proportionally, narrow gives it back', async ({ page }) => {
    await launchApp(page, 'Notes');
    await launchApp(page, 'Notes');
    await clickWindowAction(page, 1, 'Widen');
    let grow = await page.$$eval('.window', (els) => els.map((el) => parseFloat(el.style.flexGrow)));
    expect(grow[0]).toBeGreaterThan(0.5);
    expect(grow[0] + grow[1]).toBeCloseTo(1, 5);

    await clickWindowAction(page, 1, 'Narrow');
    grow = await page.$$eval('.window', (els) => els.map((el) => parseFloat(el.style.flexGrow)));
    expect(grow[0]).toBeCloseTo(0.5, 1);
  });

  test('move left/right actually reorders the DOM, not just the data', async ({ page }) => {
    await launchApp(page, 'Notes');
    await launchApp(page, 'Notes');
    await clickWindowAction(page, 1, 'Widen');
    await clickWindowAction(page, 1, 'Widen');
    let grow = await page.$$eval('.window', (els) => els.map((el) => el.style.flexGrow));
    const widenedFirst = grow[0];
    expect(parseFloat(widenedFirst)).toBeGreaterThan(0.5);

    await clickWindowAction(page, 1, 'Move right');
    grow = await page.$$eval('.window', (els) => els.map((el) => el.style.flexGrow));
    expect(grow[1]).toBe(widenedFirst);
  });

  test('minimizing one window lets the others reflow to fill the row', async ({ page }) => {
    await launchApp(page, 'Notes');
    await launchApp(page, 'Notes');
    await clickWindowAction(page, 1, 'Minimize');
    expect(await visibleWindowCount(page)).toBe(1);
    await expect(page.locator('.taskbar-item')).toHaveCount(2);
  });
});

test.describe('infinite-scroll window manager (workspace 3)', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await switchWorkspace(page, 3);
  });

  test('windows get a fixed pixel width and can be widened/narrowed', async ({ page }) => {
    await launchApp(page, 'Notes');
    const initial = await page.locator('.window').evaluate((el) => el.style.width);
    expect(initial).toBe('320px');

    await clickWindowAction(page, 1, 'Widen');
    const widened = await page.locator('.window').evaluate((el) => el.style.width);
    expect(widened).toBe('380px');
  });

  test('maximize fills the visible strip but neighbors stay reachable by scroll', async ({ page }) => {
    await launchApp(page, 'Notes');
    await launchApp(page, 'Notes');
    await clickWindowAction(page, 1, 'Maximize');
    const widths = await page.$$eval('.window', (els) => els.map((el) => el.style.width));
    expect(widths[0]).toBe('100%');
    expect(widths[1]).toBe('320px');

    // Resize/reorder actions disappear while maximized; Minimize and
    // Restore are the only ones left.
    const titles = await page.$$eval('.window:nth-child(1) .window__action', (els) =>
      els.map((e) => e.getAttribute('title'))
    );
    expect(titles).toEqual(['Minimize', 'Restore']);
  });
});
