# Security Policy

## Supported versions

Only the latest release receives security fixes.

## Reporting a vulnerability

Please **do not** open a public issue for security problems. Use GitHub's
private vulnerability reporting (“Report a vulnerability” under the Security
tab) on this repository. You'll get an acknowledgment within 72 hours.

## Scope notes

- API keys are stored in the OS keychain (`keyring`), never in config files or
  logs. If you find a code path that leaks a secret to disk or a log line,
  that's a vulnerability — report it.
- Simorgh sends audio/text to third-party APIs **only** when you explicitly
  select an API engine. The default local stack is offline.
- The updater verifies signatures via Tauri's built-in updater keys.
