# Airlock

**Private AI on your own computer.** Chat, dictate, read aloud, transcribe, make images and keep it all in one library. Nothing leaves your machine unless you say so.

Airlock is a desktop app by [Nexonauts](https://github.com/kanakkholwal). Models run locally, your files stay local, and every step an assistant takes on your behalf is visible and asks before it touches anything private.

> **Pre-release.** Airlock is early. Windows is the platform it is built and tested on. macOS and Linux builds are produced but not yet tested. Expect rough edges, and please open an issue when you find one.

## What it does

- **Chat** with a local model. Replies stream as they are written, with reasoning shown separately. Attach images to a model that can see them. Models load when you need them and free their memory when idle.
- **Dictation** anywhere. Hold a shortcut, speak, and the text is pasted into the app you are using. A history keeps every dictation, so nothing is lost if a paste goes astray.
- **Transcribe files.** Drop in a podcast, a meeting or a voice memo. Turn on timestamps to read it by time, and copy it out as plain text, timestamped text, SRT or VTT.
- **Read aloud and a Voice page.** Natural voices run on your computer. Pause, resume, change voice and speed while it speaks.
- **Make images.** Describe a picture and it is made on your GPU. A gallery with a large zoomable viewer keeps what you made.
- **A private library.** Recordings, transcripts, notes, summaries and images in one place. Everything starts private. Summarize any text item as a job that keeps its progress if stopped.
- **An assistant with permissions.** Mention library items in chat with `@`. The model sees only a short preview. To read more it must call a tool, and you approve it. It can search the library, summarize, transcribe a file from a folder you shared, and save notes. There is no "approve everything" switch.

## Privacy

- Models, transcription, voices and images all run on this computer.
- The library marks every item **private** by default. Private items never go to an online model, and for shared items an online model gets a short preview unless you grant more.
- Every read of your text by a model goes through one gate and is written to a local log, `privacy-log.jsonl`, in the app data folder.
- The assistant can only open files inside folders you share. None are shared until you add one.
- Nothing is uploaded. The app talks to the internet to download engines and models you ask for, and to check nothing else.

Online model providers are planned as an opt-in. When they arrive, the same rules apply.

## Requirements

- Windows 10 or 11, 64-bit. A GPU is optional and helps with chat and images.
- 16 GB of RAM is comfortable. A 4 GB GPU runs the default small chat model and the image model.
- Disk space for what you download: a small chat model is about 1 GB, the image model about 6.5 GB, the voice pack about 150 MB.

## Install

Download the installer from the [Releases](https://github.com/kanakkholwal/airlock/releases) page.

This build is not code-signed yet, so Windows SmartScreen will warn the first time you open it. Choose **More info**, then **Run anyway**.

On first run Airlock offers a one-button setup that downloads a chat engine and a small model. Engines and models are fetched on your action, checked against published checksums, and stored in your app data folder.

## Build from source

You need Node 22, [pnpm](https://pnpm.io), and a [Rust toolchain](https://rustup.rs). On Windows, also the Visual Studio C++ build tools. The Vulkan SDK is optional and only needed for GPU dictation.

```sh
pnpm install
pnpm app:dev:cpu     # run the app (add the Vulkan SDK and use pnpm app:dev for GPU dictation)
pnpm app:build:cpu   # build an installer
pnpm check           # type check the interface
pnpm test            # interface tests
pnpm app:test        # Rust tests
```

The first Rust build compiles the speech engine from source and takes a while.

## Make it yours

What can be downloaded is data, not code. The model, image, voice and engine lists are JSON files in [`src-tauri/catalog`](src-tauri/catalog). To add or replace an entry without rebuilding, put a file with the same name in `catalog/` inside the app data folder (`%APPDATA%\com.nexonauts.airlock`). Speech models can be added the same way with `speech-models.json`.

## How it is built

- [Tauri 2](https://tauri.app) with a Rust backend and a SvelteKit interface, using [Baby UI](https://baby-ui.nexonauts.com) components.
- Chat runs on [llama.cpp](https://github.com/ggml-org/llama.cpp), images on [stable-diffusion.cpp](https://github.com/leejet/stable-diffusion.cpp), read-aloud on [sherpa-onnx](https://github.com/k2-fsa/sherpa-onnx) with [Kokoro](https://huggingface.co/hexgrad/Kokoro-82M), and speech to text on transcribe.cpp. Each engine runs as its own process, so a crash in one cannot take the app down.
- Models are downloaded on demand and never bundled. Each model keeps its own license, shown next to it in the app.

## Roadmap

Workflows that chain steps together, clean-up of recorded audio, video dubbing, online model providers behind the same privacy rules, and an in-app updater.

## License

MIT. See [LICENSE-HANDY-MIT](LICENSE-HANDY-MIT) for the notice that covers the code this project builds on. Models and engines are separate works under their own licenses.
