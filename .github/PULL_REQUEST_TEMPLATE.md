## What changed and why

## Checks

- [ ] The title follows Conventional Commits (`feat:`, `fix:`, `docs:` ...)
- [ ] `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` and `npm run build` pass
- [ ] New logic has a test, and a bug fix has a test that fails without the fix
- [ ] Anything touching firewall teardown, socket permissions, config names or config values has been read against "Things that need care" in CONTRIBUTING.md
- [ ] Docs updated if behaviour, settings or the config format changed
