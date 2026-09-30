# Aquos Mockup

UI mockup / prototyping playground for the Aquos desktop. Rust + [Leptos](https://leptos.dev) (CSR), compiled to `wasm32-unknown-unknown` and built/served with [Trunk](https://trunkrs.dev).

## Build & run

```
cargo check --target wasm32-unknown-unknown   # fast type-check
trunk serve                                    # dev server with hot rebuild, http://127.0.0.1:8080
trunk build                                    # production build -> dist/
```

## Tests

End-to-end Playwright specs live in `e2e/`. `webServer` in `e2e/playwright.config.js` builds and serves the app automatically — no need to run `trunk serve` yourself first.

```
cd e2e
npm test        # playwright test
npm run test:headed / test:ui   # debugging
```

## Critical constraints

- **Kernel state only through `syscall()`.** `os::kernel` owns all process/window/settings/filesystem state behind `os::kernel::syscall()` (see `src/os/kernel/mod.rs`). Add new capabilities as a new `Syscall`/`SyscallResult` variant, not as new `pub` state or a side-channel function — if it isn't a variant, nothing outside the kernel can do it.
- **Window managers/workspaces are policy, not kernel.** `os::managers`, `os::window_manager`, `os::workspace` sit on top of the kernel's identity/focus primitives; they must not be folded into `os::kernel` itself.
- **`os::kernel::init()` must run first.** Called once from the root component (`main.rs`) before anything else touches kernel state, so `KERNEL`'s lazily-initialized signals are owned by the permanent root scope rather than some shorter-lived one.
- **Apps register themselves, no central match.** Every app is a struct implementing the `App` trait (`src/app/mod.rs`) added to the `ALL` list — don't add a new enum variant or match arm for apps.
- **The filesystem is real, not mocked.** `src/os/kernel/fs.rs`'s tree is generated at compile time by `build.rs` from `./fs`, with file contents baked in via `include_bytes!` (no runtime server needed to browse it). Every file and folder under `fs/` must have a sibling `<name>.meta` file with `created:`/`modified:` fields — the build fails otherwise. 