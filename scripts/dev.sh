#!/usr/bin/env bash
# Run the app in development mode (Tauri dev server + Vite HMR).
set -euo pipefail
cd "$(dirname "$0")/.."
(cd ui && pnpm install && pnpm dev &)
cargo tauri dev
