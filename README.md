# mascot-render-server

Desktop mascot (simplified version). Written in Rust.

## Features
- Easy installation. Just place the zip file.
- Effortless editing. Intuitively change outfits and poses using the TUI.
- Playful reactions. Click the head and...
- Simplified version. It has limited features, so a playful spirit is required.

## Install

Rust is required.

```
cargo install --force --git https://github.com/cat2151/mascot-render-server mascot-render-server psd-viewer-tui mascot-render-status-tui
```

## Preparation

Please place the following three zip files into `C:/Users/<YOUR NAME>/AppData/Local/mascot-render-server/assets/zip/`.

By Sakamoto Ahiru:
- `ずんだもん立ち絵素材2.3.zip`
- `ずんだもん立ち絵素材V3.2.zip`
- `ずんだもん立ち絵素材改1.1.1.zip`

## Run

```
psd-viewer-tui
```

- The following will be performed automatically:
    - Unpacking zip files
    - Analyzing PSD files contained in the zip
    - Analyzing layers within the PSD
    - Displaying the desktop mascot
        - It will be displayed with default layers.

- Changing outfit or pose layers will alter its appearance.
- If you register favorites in the TUI, the server will refer to the cache and shuffle playback to a different favorite every minute.

- For detailed features, please refer to the help screen.

## Configuration
- mascot-render-server.toml
    - always_idle_sink
        - Initial value is `true`.
        - If true, a small IdleSink breathing-like motion will be continuously played for UX verification.
    - always_bend
        - Initial value is `true`.
        - If true, a left/right bend motion for UX verification will be continuously played.
    - bend
        - `amplitude_ratio` allows you to specify the bend width as a ratio to the mascot image width. The default is `0.0075`.
    - idle_sink
        - This is the IdleSink breathing setting specifically for always_idle_sink.
        - By default, it sinks and lifts more gently than a regular squash_bounce, and the tempo changes gradually in sync with the median fluctuation of the blink interval.
        - `sink_amount` and `lift_amount` allow individual adjustment of the exhalation and inhalation poses.

## Architecture
- Modular
    - Emphasizing reusability, it is implemented by dividing it into small crates, each with its own responsibility.
- Server
    - Dedicated to being a visual-render-server. Responsibilities such as orchestration of dialogue and motion are intended to be handled by a separate, higher-level application.
- Sixel
    - For fallback in case the desktop mascot doesn't work, a preview of the mascot is displayed in the terminal.
- PSDTool
    - Supports [PSDTool](https://oov.github.io/psdtool/manual.html)'s extended formats "Radio Button Conversion" and "Forced Display", enabling comfortable editing.
- Format
    - Management formats like ghost or shell have not been implemented.
        - Currently, it has only been tested with Zundamon character portrait materials created by Sakamoto Ahiru.
        - It's possible to use it generically by rewriting the toml file, but this is unconfirmed.

## About vendor/

- `vendor/rawpsd` is a vendored copy of the [rawpsd](https://github.com/wareya/rawpsd-rs) library, with AI-assisted fixes for bugs.
- It used to panic when trying to read PSDs handled by mascot-render-server, so Codex CLI was used to fix that.
- Refactored due to CI considerations. [PR 17](https://github.com/cat2151/mascot-render-server/pull/17#pullrequestreview-3988754980)

## Caveats
- This is a personal application, so it's not intended for others to use. If you want similar functionality, we recommend cloning it or creating your own.
- Destructive changes are frequent. Even if someone builds related functionality, it might become unusable the next day.

## Goals of this Application
- PoC. To demonstrate that a useful personal application can be created with Codex CLI (Codex Plus 30-day free trial) (demonstrated).
- PSD. To easily handle PSDs in Rust.
- Desktop Mascot. To easily implement a desktop mascot in Rust.
- Eye blinking and lip-syncing.
- Server. To easily control the desktop mascot from other applications via an HTTP REST API.

## What this Application Does NOT Aim For (Out of Scope)
- Establishing a new high-functional general-purpose desktop mascot common standard, developing a governance system for it, and continuous operation.
- Support. Responding to requests or suggestions.