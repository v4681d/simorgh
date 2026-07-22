## What & why

<!-- Summary of the change and motivation. Link issues: Fixes #123 -->

## How

<!-- Notable implementation decisions -->

## Checklist

- [ ] `cargo fmt --all` and `cargo clippy --workspace --all-targets -- -D warnings` pass
- [ ] `cargo test --workspace` passes
- [ ] New engines implement the trait fully (partials/finals contract, `self_generated` for TTS) and are registered in the factory + UI catalog
- [ ] Docs updated if behavior changed (`docs/`, README)
- [ ] No secrets in config files or code — keys go through the OS keychain
