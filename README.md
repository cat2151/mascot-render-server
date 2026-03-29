# mascot-render-server

A desktop mascot (simplified version) written in Rust.

## Features
- Easy installation. Just place the zip file.
- Effortless editing. Intuitively change outfits and poses using the TUI.
- Amusing reactions. Click the head and...
- This is a simplified version. It has limited features, so a playful spirit is required.

## install

Rust is required.

```
cargo install --force --git https://github.com/cat2151/mascot-render-server mascot-render-server psd-viewer-tui
```

## Setup

Please place the following three zip files in `C:/Users/<YOUR NAME>/AppData/Local/mascot-render-server/assets/zip/`.

Created by SAKAMOTO Ahiru:
- `ずんだもん立ち絵素材2.3.zip`
- `ずんだもん立ち絵素材V3.2.zip`
- `ずんだもん立ち絵素材改1.1.1.zip`

## Execution

```
psd-viewer-tui
```

- The following will be performed automatically:
    - Extraction of zip files
    - Analysis of PSD files within the zip
    - Analysis of layers within the PSD
    - Display of the desktop mascot
        - It will be displayed with default layers.

- Changing the outfit or pose layers will alter its appearance.
- If you register favorites in the TUI, the server will reference the cache and shuffle playback to a different favorite every minute.

- For detailed features, please refer to the on-screen help.

## Configuration
- mascot-render-server.toml
    - always_idle_sink
        - Defaults to `true`.
        - If true, a small IdleSink breathing-like motion will be constantly played for UX verification.
    - always_bend
        - Defaults to `true`.
        - If true, left/right bending for UX verification will be constantly played.
    - bend
        - The bend width can be specified as a ratio to the mascot image width using `amplitude_ratio`. The default is `0.0075`.
    - idle_sink
        - IdleSink breathing settings specifically for `always_idle_sink`.
        - By default, it sinks and lifts more gently than regular squash_bounce, and the tempo changes subtly in sync with the median fluctuation of the blink interval.
        - You can individually adjust the exhalation and inhalation poses using `sink_amount` and `lift_amount`.

## Architecture
- Modular
    - Emphasizing reusability, it is implemented by dividing into small crates, each responsible for specific tasks.
- server
    - Dedicated to being a visual render server. The orchestration of dialogue and motion is intended to be handled by a separate, higher-level application.
- Sixel
    - For fallback in case the desktop mascot doesn't move, a preview of the mascot is displayed in the terminal.
- PSDTool
    - Supports the extended formats of [PSDTool](https://oov.github.io/psdtool/manual.html), "Radio button conversion" and "Forced display," enabling comfortable editing.
- format
    - Management formats like ghost or shell have not been implemented.
        - Currently, it has only been tested with Zundamon standing illustration materials created by SAKAMOTO Ahiru.
            - It might be possible to use it generically by rewriting the toml file, but this is unconfirmed.

## About vendor/

- `vendor/rawpsd` is a vendored copy of the [rawpsd](https://github.com/wareya/rawpsd-rs) library, with AI-assisted fixes for bugs.
- It used to panic when trying to read PSDs handled by mascot-render-server, so this part was fixed by Codex CLI.
- Refactored due to CI considerations. [PR 17](https://github.com/cat2151/mascot-render-server/pull/17#pullrequestreview-3988754980)

## Prerequisites
- This application is for personal use and not intended for others. If you want similar functionality, we recommend cloning or creating your own.
- Destructive changes are made frequently. Even if someone were to build related functionality, it might become unusable the next day.

## Goals of this application
- PoC. To demonstrate (and has demonstrated) that useful personal applications can be created with Codex CLI (Codex Plus 30-day free trial).
- PSD. Easy handling of PSDs in Rust.
- Desktop Mascot. Easy implementation of desktop mascots in Rust.
- Blinking eyes and lip-syncing.
- Server. Easy manipulation of the desktop mascot from other applications via an HTTP REST API.

## Out of Scope
- Establishing new highly functional general-purpose desktop mascot common standards, developing a governance system for them, and continuous operation.
- Support. Responding to requests or suggestions.
