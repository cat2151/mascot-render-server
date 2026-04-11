# mascot-render-server

A simplified desktop mascot, written in Rust.

## Features
- Easy installation. Just place the zip file.
- Effortless editing. Intuitively change outfits and poses using the TUI.
- Amusing reactions. Click its head for...
- This is a simplified version. It has limited features, so a playful spirit is required.

## Install

Rust is required.

```
cargo install --force --git https://github.com/cat2151/mascot-render-server mascot-render-server psd-viewer-tui mascot-render-status-tui
```

## Preparation

Please place the following three zip files in `C:/Users/<YOUR NAME>/AppData/Local/mascot-render-server/assets/zip/`.

Created by Sakamoto Ahiru:
- `ずんだもん立ち絵素材2.3.zip`
- `ずんだもん立ち絵素材V3.2.zip`
- `ずんだもん立ち絵素材改1.1.1.zip`

## Execution

```
psd-viewer-tui
```

- The following actions are performed automatically:
    - Unzips the asset files.
    - Analyzes PSD files contained in the zip archives.
    - Analyzes layers within the PSD files.
    - Displays the desktop mascot.
        - Displayed with default layers.

- Changing the outfit or pose layers will alter its appearance.
- If you register favorites in the TUI, the server will reference its cache and shuffle playback to a different favorite every minute.

- Refer to the on-screen help for detailed features.

## Configuration
- mascot-render-server.toml
    - always_idle_sink
        - The initial value is `true`.
        - If `true`, a small IdleSink breathing-like motion is constantly played for UX verification.
    - always_bend
        - The initial value is `true`.
        - If `true`, the left/right bend motion for UX verification is constantly played.
    - bend
        - The bend width can be specified as a ratio to the mascot image width using `amplitude_ratio`. The default is `0.0075`.
    - idle_sink
        - IdleSink breathing settings specifically for `always_idle_sink`.
        - By default, it sinks and lifts more gently than a regular squash_bounce, and the tempo changes gradually in sync with the fluctuation of the median blink interval.
        - You can individually adjust the exhalation and inhalation poses using `sink_amount` and `lift_amount`.

## Architecture
- Modular
    - Emphasizing reusability, it's implemented by dividing it into small crates, each responsible for specific tasks.
- Server
    - Dedicated to being a visual render server. Responsibilities such as orchestration of dialogue and motion are intended to be handled by separate, higher-level applications.
- Sixel
    - For fallback in case the desktop mascot fails to run, a preview of the mascot is displayed in the terminal.
- PSDTool
    - Supports the extended formats of [PSDTool](https://oov.github.io/psdtool/manual.html) – 'radio buttonization' and 'forced display' – to enable comfortable editing.
- Format
    - Management formats like 'ghost' or 'shell' are not yet implemented.
        - Currently, it has only been tested with Zundamon character portrait materials created by Sakamoto Ahiru.
            - While there's a possibility it could be used generally by rewriting the toml file, this is unconfirmed.

## About `vendor/`

- `vendor/rawpsd` is a vendored copy of the [rawpsd](https://github.com/wareya/rawpsd-rs) library, with AI-assisted fixes applied to address bugs.
- It was panicking when trying to read PSDs handled by mascot-render-server, so Codex CLI was used to fix that.
- Refactored due to CI considerations. [PR 17](https://github.com/cat2151/mascot-render-server/pull/17#pullrequestreview-3988754980)

## Prerequisites
- This application is for personal use and not intended for others. If you desire similar functionality, cloning or developing your own is recommended.
- Breaking changes are frequent. Even if someone builds related functionality, it might become unusable the very next day.

## Goals of This Application
- PoC: To demonstrate (and has demonstrated) that useful personal applications can be created with Codex CLI (Codex Plus 30-day free trial).
- PSD: To easily handle PSDs in Rust.
- Desktop Mascot: To easily implement a desktop mascot in Rust.
- Eye blinking and lip-sync.
- Server: To easily control the desktop mascot from other applications via an HTTP REST API.

## Non-Goals (Out of Scope)
- Formulation of a new high-functionality general-purpose desktop mascot standard, establishment of a governance system for it, and continuous operation.
- Support: Responding to requests or suggestions.
