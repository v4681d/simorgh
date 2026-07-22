# Adding engines

Every pipeline stage is a trait object created by a small factory. To add an
engine you implement one trait and register one line.

## 1. Implement the trait

- STT: `simorgh_core::traits::SttEngine` (`crates/engines-stt`)
- Translation: `simorgh_core::traits::Translator` (`crates/engines-mt`)
- TTS: `simorgh_core::traits::TtsEngine` (`crates/engines-tts`)

```rust
#[async_trait]
impl SttEngine for MyEngine {
    fn id(&self) -> &'static str { "my-engine" }
    fn display_name(&self) -> String { "My Engine".into() }
    fn is_local(&self) -> bool { false }
    async fn start_session(&self, lang: Lang, audio_rx: Receiver<AudioFrame>,
                           transcript_tx: Sender<TranscriptSegment>) -> Result<SttSession> { … }
}
```

Rules of the road:

- Emit **partials** (`is_final: false`) as often as you like; emit exactly one
  **final** per segment. Finals must never change afterwards.
- TTS engines must set `self_generated: true` on every output `AudioFrame` —
  this drives the anti-feedback gate.
- Return `Error::Engine { fix: Some(…) }` with an actionable fix ("Enter your
  API key in Settings → Engines") — the UI shows it verbatim.
- Read base URL / API key / proxy from `EngineChoice`; secrets come from the
  OS keychain via the crate-local `api_key()` helper, never from config files.

## 2. Register it in the factory

Add a match arm in the crate's `create()` in `src/lib.rs`:

```rust
"my-engine" => Ok(Arc::new(my_engine::MyEngine::new(choice)?)),
```

## 3. Expose it in the UI

Add an entry to `CATALOG` in `ui/src/screens/Engines.tsx`.

## 4. Test it

Use the mock engines and recorded fixtures as templates
(`crates/app/tests/pipeline_integration.rs`). Engines must be testable without
network — gate real-API tests behind an env var.

---

## The custom WebSocket STT protocol

The built-in `custom` STT engine speaks a minimal protocol so you can bridge
any recognizer:

- **Client → server:** binary WebSocket messages of raw PCM audio,
  **16 kHz, mono, s16le**, sent as capture happens.
- **Server → client:** JSON text messages:

```json
{ "text": "transcribed text so far", "final": false }
```

Send `"final": true` to close a segment (the engine then starts a new one).
Anything else on the socket is ignored. Point the engine at your server with
the Base URL field (`ws://` or `wss://`).

A reference server stub lives in `docker/` (see `Dockerfile.engines`) — it
wires whisper.cpp's server into this protocol.
