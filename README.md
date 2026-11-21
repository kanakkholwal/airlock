# Airlock

Private AI on your own computer. Chat, dictate, transcribe, read aloud and make images with local models. Nothing leaves your machine unless you say so.

> Pre-release. Built and tested on Windows only.

## Features

- Chat with local models, with image input
- System-wide dictation, and file transcription with timestamps (copy as text, SRT or VTT)
- Read aloud with natural voices
- Image generation
- A private library of everything you make, with an assistant that asks before it reads or saves anything

## Privacy

- Models, speech and images all run locally.
- Library items are private by default and never reach an online model. Online providers are planned and will follow the same rule.
- The assistant sees a short preview of what you mention with `@`. Reading more needs your approval, and each read is logged locally.

## Install

Get the installer from [Releases](https://github.com/kanakkholwal/airlock/releases). It is unsigned, so SmartScreen will warn on first run. Engines and models download on first use.

## Build

Needs Node 22, pnpm and Rust.

```sh
pnpm install
pnpm app:dev:cpu
pnpm app:build:cpu
```

Model, voice and engine lists are JSON in [`src-tauri/catalog`](src-tauri/catalog). Add or override entries in `catalog/` inside the app data folder.

## License

MIT. See [LICENSE-HANDY-MIT](LICENSE-HANDY-MIT).
