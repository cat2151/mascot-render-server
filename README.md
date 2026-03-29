# mascot-render-server

Desktop mascot (simplified version). Written in Rust.

## Features
- Easy installation. Just place the zip file.
- Effortless editing. Intuitively change outfits and poses using the TUI.
- Amusing reactions. Click the head to...
- Simplified version. It has limited features, so a playful spirit is required.

## install

Rust is required.

```
cargo install --force --git https://github.com/cat2151/mascot-render-server mascot-render-server psd-viewer-tui
```

## Preparation

Please place the following three zip files in `C:/Users/<YOUR NAME>/AppData/Local/mascot-render-server/assets/zip/`.

Created by Ahiru Sakamoto
- `ずんだもん立ち絵素材2.3.zip`
- `ずんだもん立ち絵素材V3.2.zip`
- `ずんだもん立ち絵素材改1.1.1.zip`

## Execution

```
psd-viewer-tui
```

- The following will happen automatically:
    - Extraction of zip files
    - Analysis of PSD files within the zip
    - Analysis of layers within the PSD
    - Display of the desktop mascot
        - Displayed with default layers

- Changing outfit or pose layers will alter its appearance.
- If you register favorites in the TUI, the server will refer to the cache and shuffle playback to a different favorite every minute.

- For detailed features, please refer to the on-screen help.

## Settings
- mascot-render-server.toml
    - always_idle_sink
        - If `true`, a small IdleSink breathing-like motion will constantly play for UX verification.
    - always_bend
        - If `true`, constant left/right bending will play for UX verification.
    - bend
        - With `amplitude_ratio`, you can specify the bend width as a ratio to the mascot image width. The default is `0.015`.
    - idle_sink
        - IdleSink breathing settings specific to `always_idle_sink`.
        - By default, it sinks and rises more gently than normal `squash_bounce`, and the tempo gradually changes in sync with the fluctuations in the median blink interval.
        - With `sink_amount` and `lift_amount`, you can individually adjust the poses for the exhale and inhale sides.

## Architecture
- Modular
    - Implemented by dividing into small crates for each responsibility, emphasizing reusability.
- server
    - Dedicated to being a visual-render-server. Responsibilities like orchestrating dialogue and motion are intended to be held by a separate, higher-level application.
- Sixel
    - For fallback in case the desktop mascot doesn't work, a preview of the mascot is displayed in the terminal.
- PSDTool
    - Supports [PSDTool](https://oov.github.io/psdtool/manual.html)'s extended formats "radio button conversion" and "forced display", enabling comfortable editing.
- format
    - Management formats like ghost or shell have not been implemented.
        - Currently, it has only been tested with Zundamon standing pose materials created by Ahiru Sakamoto.
            - It might be possible to use it generically by rewriting the toml file, but this is unconfirmed.

## About `vendor/`

- `vendor/rawpsd` is a vendored copy of the [rawpsd](https://github.com/wareya/rawpsd-rs) library, with AI-assisted fixes for bugs.
- It was panicking when trying to read PSDs handled by `mascot-render-server`, so I had Codex CLI fix that.
- Refactored due to CI circumstances. [PR 17](https://github.com/cat2151/mascot-render-server/pull/17#pullrequestreview-3988754980)

## Premise
- This is an application for personal use, not intended for others. If you want similar functionality, I recommend cloning or creating your own.
- Destructive changes are made frequently. Even if someone were to build related features, they might become unusable the very next day.

## What this application aims for
- PoC. To demonstrate (and has demonstrated) that useful personal applications can be created with Codex CLI (Codex Plus 30-day free trial).
- PSD. Easily handle PSDs in Rust.
- Desktop Mascot. Easily implement a desktop mascot in Rust.
- Blinking and lip-syncing.
- Server. Easily control the desktop mascot from other applications via HTTP REST API.

## What is not aimed for (out of scope)
- Formulation of new high-functional general-purpose desktop mascot common standards, establishment of governance structure for it, and continuous operation.
- Support. Responding to requests and suggestions.
