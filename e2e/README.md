# Aquos mockup — end-to-end tests

Playwright specs that drive the actual app in a real (headless) browser —
click buttons, drag windows, type into inputs, and assert on what
rendered. This is what verified every feature added to the mockup;
consolidated here so it's one command instead of a pile of ad-hoc scripts.

## Run everything

```sh
cd e2e
npm install      # once
npm test
```

That single command builds the app, starts `trunk serve`, runs every spec,
and shuts the server down again. No separate build/serve step needed.

Other useful commands:

```sh
npm run test:headed   # watch it click through the app in a real window
npm run test:ui       # Playwright's interactive UI mode
npx playwright test tests/apps.spec.js          # just one file
npx playwright test -g "drag"                   # just tests matching a name
npm run report         # open the HTML report from the last run
```

## Layout

- `tests/helpers.js` — shared page-interaction helpers (open the start
  menu, launch an app, drag a window, install/uninstall via the App
  Store, ...). Reach for these before writing raw selectors.
- `tests/desktop.spec.js` — shell basics: start menu, fullscreen, clock.
- `tests/workspaces.spec.js` — switching workspaces, per-workspace layout.
- `tests/window-management.spec.js` — the three window managers (drag,
  maximize, tiling resize/reorder, infinite-scroll resize/reorder,
  minimize/restore, close).
- `tests/apps.spec.js` — each app's own behavior (Notes, File Explorer,
  Contacts, Calculator, Program Manager).
- `tests/app-store.spec.js` — install/uninstall, including the "remove
  personalized data" flow.
- `tests/settings.spec.js` — every settings control actually changes the
  behavior it claims to, not just the stored value.

## Adding a test

Add a `test(...)` to the relevant spec file (or start a new
`whatever.spec.js` — anything matching `tests/*.spec.js` is picked up
automatically). Each test gets a fresh `page.goto('/')`, i.e. a clean
reload of the app with no windows open — Leptos state lives entirely in
that page's WASM instance, so there's no cross-test state to reset by
hand.

Two things that have bitten these tests before, worth knowing:

- **Floating windows overlap.** If a test opens two apps in the default
  (floating) workspace and needs to click something in the *first* one,
  the second one (launched later, cascaded on top) may cover it. Either
  focus the window first or, often simpler, run that part of the test in
  the tiling workspace (`switchWorkspace(page, 2)`), where windows never
  overlap.
- **Prefer real interactions over `el.click()`/`el.value =`.** Coordinate
  clicks and Playwright's `fill()` go through the same event path a user
  would; this app has previously had bugs (windows silently swallowing
  clicks) that only a real click sequence reproduces.
