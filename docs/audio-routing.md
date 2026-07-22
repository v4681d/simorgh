# Audio routing

Simorgh needs two special audio paths: **loopback** (hearing what your
computer plays) and a **virtual microphone** (feeding translated speech into
Zoom/Discord/Meet). This is the per-OS state of the world.

## Linux (PulseAudio / PipeWire) — fully automatic

**Loopback:** every output has a `.monitor` source; Simorgh lists them as
loopback devices automatically. Nothing to install.

**Virtual mic:** Simorgh creates it programmatically on startup of Pipeline B:

```sh
pactl load-module module-null-sink sink_name=simorgh_mic \
      sink_properties=device.description="Simorgh Microphone"
pactl load-module module-remap-source master=simorgh_mic.monitor \
      source_name=simorgh_mic_src source_properties=device.description="Simorgh Microphone"
```

Pick **“Simorgh Microphone”** as the mic in your meeting app. Simorgh removes
the modules on exit.

## Windows

**Loopback:** WASAPI loopback — Simorgh opens render devices in loopback mode
and lists them as “… (loopback)”. Nothing to install.

**Virtual mic:** Windows has no built-in virtual audio device. Simorgh detects
[VB-Cable](https://vb-audio.com/Cable/) and guides you through installing it
(one-time, free). Then:

- Simorgh plays TTS into **CABLE Input**.
- Your meeting app uses **CABLE Output** as its microphone.

## macOS

**Loopback and virtual mic:** CoreAudio allows neither without a driver.
Simorgh detects [BlackHole](https://github.com/ExistentialAudio/BlackHole)
(free, open source) and walks you through setup:

1. `brew install blackhole-2ch`
2. Create a **Multi-Output Device** (your speakers + BlackHole) in Audio MIDI
   Setup so you still hear the audio Simorgh is subtitling.
3. Simorgh captures from BlackHole for loopback and plays TTS into a second
   BlackHole device that your meeting app uses as its mic.

## Feedback prevention

Regardless of routing, Pipeline A never transcribes Simorgh's own voice: TTS
frames are tagged `self_generated` and a global speaking flag gates the
subtitle pipeline while TTS plays. If you physically route speakers into your
mic, use push-to-talk mode.

## Troubleshooting

| Symptom | Fix |
|---|---|
| No loopback devices listed (Linux) | `pactl list sources short` — ensure PipeWire/Pulse is running |
| “Simorgh Microphone” missing after reboot (Linux) | It's created on demand; just start Pipeline B |
| Meeting app doesn't list CABLE Output (Windows) | Reinstall VB-Cable, then restart the meeting app |
| Echo of your own translated voice | Enable push-to-talk; check you didn't select the virtual mic as Simorgh's input |
