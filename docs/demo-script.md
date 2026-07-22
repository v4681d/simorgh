# Demo script (≈ 3 minutes)

Goal: show both pipelines live, engine swapping, and the overlay.

## Setup (before recording)

- Linux machine (routing is fully automatic there) with `ggml-small.bin`
  downloaded and a local Ollama running `qwen2.5:7b` (or any chat model).
- A YouTube clip with clear English speech queued up.
- A test meeting (e.g. meet.jit.si) open in a second browser.

## Script

1. **Cold open (20 s).** Launch Simorgh. First-run wizard appears (light
   theme). Click through: pick the speaker monitor as loopback — point out the
   live meter proving audio flows — pick the mic, finish.
2. **Pipeline A (60 s).** On the Dashboard, toggle *Listen & subtitle* on.
   Play the YouTube clip. Show the overlay: dimmed English line, large Persian
   line. Point at provisional text solidifying: “notice the lighter text —
   that's the retranslation policy; once a sentence is final it locks and
   never changes.” Show the `X.Xs behind live` badge and per-stage latency
   badges on the card. Drag the overlay; scroll to resize the font.
3. **Engine swap (30 s).** Open Engines. Flip STT from Local → API → back
   while audio plays; subtitles continue after a brief reconnect — no restart.
4. **Pipeline B (60 s).** Toggle *Speak & translate* on. In the test meeting,
   select **Simorgh Microphone** as mic. Speak a Persian sentence, pause —
   after the pause, the English TTS voice speaks into the meeting. Say:
   “Simorgh deliberately waits for the end of the clause — Persian puts the
   verb last, so speaking earlier would be wrong. The latency badge is honest
   about that.”
5. **Close (10 s).** Show History → Export SRT. “Local-first, open source,
   Apache-2.0 — and every stage is a trait you can swap.”
