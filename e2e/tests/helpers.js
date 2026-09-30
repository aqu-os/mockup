// Shared helpers for driving the Aquos desktop mockup. Every spec file
// starts from a fresh `page.goto('/')`, so these assume nothing is open
// yet unless a test explicitly launches it first.

/** Opens the start menu (bottom-left "Aquos" button). */
async function openStart(page) {
  await page.click('.menu-bar__brand');
  await page.waitForSelector('.start-menu', { timeout: 5000 });
}

/** Closes the start menu by clicking empty desktop space, not a window. */
async function closeStart(page) {
  await page.click('.desktop', { position: { x: 700, y: 700 } });
  await page.waitForTimeout(80);
}

/** Opens the start menu and launches the app with this exact title. */
async function launchApp(page, title) {
  await openStart(page);
  await page.click(`.start-menu__item:has-text("${title}")`);
}

/** Switches to workspace 1/2/3 via the workspace pills in the menu bar. */
async function switchWorkspace(page, number) {
  const pills = await page.$$('.ws-pill');
  await pills[number - 1].click();
}

/** Switches the active workspace's layout via the layout pills. Label is
 *  one of "Floating", "Tiling", "Infinite scroll" (matches the pill's
 *  title attribute). */
async function switchLayout(page, label) {
  await page.click(`.layout-pill[title="${label}"]`);
}

/** Clicks a titlebar action button (Minimize, Maximize, Widen, ...) on the
 *  nth window (1-indexed, matching CSS :nth-child) currently in the DOM. */
async function clickWindowAction(page, windowIndex, actionTitle) {
  await page.click(`.window:nth-child(${windowIndex}) .window__action[title="${actionTitle}"]`);
}

/** Drags a window by its titlebar by (dx, dy) pixels. `windowIndex` is
 *  1-indexed against the current `.window__titlebar` elements in the DOM. */
async function dragWindow(page, windowIndex, dx, dy) {
  const titlebar = (await page.$$('.window__titlebar'))[windowIndex - 1];
  const box = await titlebar.boundingBox();
  await page.mouse.move(box.x + 50, box.y + 15);
  await page.mouse.down();
  await page.mouse.move(box.x + 50 + dx, box.y + 15 + dy, { steps: 8 });
  await page.mouse.up();
}

/** Count of windows currently laid out (not minimized — `display: none`
 *  windows are excluded since they're still mounted, just hidden). */
async function visibleWindowCount(page) {
  return page.$$eval('.window', (els) => els.filter((el) => getComputedStyle(el).display !== 'none').length);
}

/** Opens the App Store and clicks Install on the row matching `title`. */
async function installApp(page, title) {
  await launchApp(page, 'App Store');
  await page.waitForSelector('.store');
  const row = await page.locator('.store-item', { hasText: title }).first();
  await row.locator('.store-item__btn--primary').click();
}

/** Opens the App Store and uninstalls the app matching `title`, optionally
 *  checking "also remove personalized data" first. */
async function uninstallApp(page, title, { removeData = false } = {}) {
  await launchApp(page, 'App Store');
  await page.waitForSelector('.store');
  const row = await page.locator('.store-item', { hasText: title }).first();
  await row.locator('.store-item__btn--danger').click();
  if (removeData) {
    await row.locator('.store-confirm__checkbox input').click();
  }
  await row.locator('.store-confirm__actions .store-item__btn--danger').click();
}

/** Opens Settings and selects a sidebar entry (an OS category label like
 *  "Desktop", or an installed app's title). */
async function openSettingsSection(page, label) {
  await launchApp(page, 'Settings');
  await page.waitForSelector('.settings');
  await page.click(`.settings__nav-item:has-text("${label}")`);
}

/** Assumes Contacts is already open with no pipeline attached (the
 *  "Connect a data source" builder showing) and wires it to Contacts
 *  Source — the fixed pipeline every Contacts test needs before the
 *  usual list/detail UI appears. */
async function connectContactsSource(page) {
  await page.waitForSelector('.contacts-sink__builder');
  await page.click('.app-picker__btn:has-text("Contacts Source")');
  await page.waitForSelector('.contacts-sink__header');
}

/** From Contacts' normal list view (a pipeline already attached), opens
 *  the pipeline editor, adds a manipulator by its title (e.g. "Sort",
 *  "Filter"), and returns to the list view — leaving the manipulator's
 *  chip open for further interaction (e.g. clicking a param choice)
 *  before the caller closes the editor themselves if needed. */
async function addManipulator(page, title) {
  await page.click('.contacts-sink__edit-btn');
  await page.waitForSelector('.contacts-sink__builder');
  await page.click(`.pipeline-chain__add-btn:has-text("${title}")`);
}

module.exports = {
  openStart,
  closeStart,
  launchApp,
  switchWorkspace,
  switchLayout,
  clickWindowAction,
  dragWindow,
  visibleWindowCount,
  installApp,
  uninstallApp,
  openSettingsSection,
  connectContactsSource,
  addManipulator,
};
