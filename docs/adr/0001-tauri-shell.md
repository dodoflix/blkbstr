# 1. Tauri as the application shell

Status: accepted

## Context

Blockbuster needs one desktop app across Linux, Windows and the BSDs, with a native-feeling
installer, a system tray, auto-update, and a Rust core that can talk to a privileged service.

## Decision

Tauri 2, Rust backend, web frontend.

## Why

Electron ships a browser per app: ~150 MB for a tool whose users may be downloading it over a
throttled connection, in a country where large downloads are conspicuous. Tauri uses the system
webview and produces single-digit-megabyte bundles.

The backend being Rust matters more than the size. The daemon has to be a small, dependency-light
privileged binary, and sharing `blkbstr-core` between it and the GUI means the config model, the
renderer and the protocol are one definition rather than two that drift.

Tauri's updater plugin consumes a GitHub-Releases-based manifest directly, which is the update
mechanism this project wants anyway.

## Costs

The system webview differs per platform — WebKitGTK, WebView2, WKWebView — so rendering bugs are
per-platform and CSS support is whatever the oldest supported OS ships. That is the price of not
bundling a browser, and it is worth it here.

## Alternatives

**Electron** — larger, and a Node backend means a second language for the privileged side or an
awkward FFI to it.

**Native per platform** — best result, three times the work, and no realistic way to keep three
implementations at feature parity in a project this size.

**GTK/Qt via Rust bindings** — smaller still, but the ecosystem for the kind of guided,
wizard-driven UI this project needs is thinner.
