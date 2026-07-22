# CLAUDE.md — Simorgh

Real-time bidirectional speech translation overlay. Rust core + React/Tauri desktop UI.
Two pipelines: **A** listen→subtitle (system audio → STT → MT → overlay), **B** speak→translate (mic → VAD → STT → MT → TTS → virtual mic).

## graphify knowledge graph

A navigable graph of this codebase lives in `graphify-out/`. **Consult it before answering codebase questions** — it encodes call graphs, trait implementations, shared data flow, and cross-cutting connections AST alone misses.

- `graphify-out/GRAPH_REPORT.md` — audit report: god nodes, community map, surprising connections, suggested questions. Read this first.
- `graphify-out/graph.json` — raw graph (779 nodes · 1023 edges · 72 communities). Query it for traversal.
- `graphify-out/graph.html` — interactive visualization, open in browser.

### How to query
```
/graphify query "<question>"            # BFS — broad context
/graphify query "<question>" --dfs      # DFS — trace a chain
/graphify path "NodeA" "NodeB"          # shortest path between concepts
/graphify explain "NodeName"            # everything connected to one node
```

### Keep it current
After code changes (new files, refactors, new engines), rebuild so the graph stays accurate:
```
/graphify . --update        # incremental — re-extracts only changed files (cheap)
/graphify .                 # full rebuild
```
Code-only changes rebuild without an LLM. Docs/papers/images need `--update` (LLM semantic re-extraction).
The watcher (`/graphify . --watch`) auto-rebuilds on code edits in the background.

### Architecture map (from the graph)
- **God nodes:** `AppState` (shared mutable hub), `main()` (bootstrap bridge, highest betweenness), `start_pipeline` (engine orchestration chokepoint).
- **Engine traits** (`crates/core/src/traits.rs`): `SttEngine`, `Translator`, `TtsEngine`, `CommitPolicy`. All providers implement these and are runtime-swappable.
  - STT: whisper_local, openai, deepgram, custom_ws, mock (`crates/engines-stt`)
  - MT: deepl, google, openai_compat, mock (`crates/engines-mt`)
  - TTS: elevenlabs, openai_tts, piper, mock (`crates/engines-tts`)
- **Commit policy** is the load-bearing design decision: Persian SOV vs English SVO word order forces clause-end commits. `RetranslationPolicy` (Pipeline A subtitles) vs `ClauseCommitPolicy` (Pipeline B voice).
- **Anti-feedback gate:** every TTS output sets `AudioFrame::self_generated = true` so it's filtered out of the subtitle loop.
- **UI** (`ui/src/screens`): Dashboard, Devices, Engines, History, Models, Overlay, Settings, Wizard. All call `api.ts` (typed Tauri IPC). `Overlay` is the frameless subtitle window; the rest render under `App`'s sidebar router.

> Note: the graph is a snapshot of the codebase at build time. If a query contradicts current source, trust the source and run `--update`.
