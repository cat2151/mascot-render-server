# mascot-render-server

A desktop mascot (simplified version) written in Rust.

## Features
- Easy installation. Just place the zip file.
- Effortless editing. Intuitively change outfits and poses using the TUI.
- Fun reactions. Click its head and...
- This is a simplified version. It has limited features, so a playful spirit is required.

## Installation

Rust is required.

```
cargo install --force --git https://github.com/cat2151/mascot-render-server mascot-render-server psd-viewer-tui
```

## Preparation

Please place the following three zip files into `C:/Users/<YOUR NAME>/AppData/Local/mascot-render-server/assets/zip/`.

Created by Sakamoto Ahiru:
- `ずんだもん立ち絵素材2.3.zip`
- `ずんだもん立ち絵素材V3.2.zip`
- `ずんだもん立ち絵素材改1.1.1.zip`

## Running

```
psd-viewer-tui
```

- The following will happen automatically:
    - Extraction of zip files
    - Analysis of PSD files contained in the zip
    - Analysis of layers within the PSD
    - Display of the desktop mascot
        - It will be displayed with default layers.

- Changing the outfit and pose layers will alter its appearance.
- If you register favorites in the TUI, the server will refer to its cache and shuffle playback to a different favorite every minute.

- For detailed features, please refer to the on-screen help.

## Settings
- `mascot-render-server.toml`
    - `always_bouncing`
        - If `true`, a small squash/stretch breathing-like motion will constantly play for UX verification.
    - `always_squash_bounce`
        - Dedicated squash/stretch settings for `always_bouncing`.
        - By default, it is slower and smaller than the regular `squash_bounce`, and its tempo drifts with the eye-blink interval median.
    - `transparent_background_click_through`
        - If `true`, it reduces "confusion from dragging empty space," although it becomes very resource-intensive.
    - `flash_blue_background_on_transparent_input`
        - If `true`, attempting to click or drag empty space will flash a blue background for one second to notify you.

## Architecture
- Modular
    - Prioritizing reusability, it's implemented by splitting into small crates, each with a specific responsibility.
- Server
    - Dedicated to being a visual render server. Responsibilities such as orchestration of dialogue and motion are intended to be handled by a separate, higher-level application.
- Sixel
    - For fallback in case the desktop mascot fails to run, a preview of the mascot is displayed in the terminal.
- PSDTool
    - Supports [PSDTool](https://oov.github.io/psdtool/manual.html)'s extended formats, "radio buttonization" and "forced display," enabling comfortable editing.
- Format
    - Management formats like ghost or shell are not yet implemented.
        - Currently, it has only been tested with Zundamon standing pose materials created by Sakamoto Ahiru.
        - It might be possible to use it generically by rewriting the toml file, but this is unconfirmed.

## About `vendor/`

- `vendor/rawpsd` is a vendored copy of the [rawpsd](https://github.com/wareya/rawpsd-rs) library, with AI-assisted fixes for bugs.
- It was panicking when trying to read PSDs handled by mascot-render-server, so Codex CLI was used to fix that.
- Refactored due to CI considerations. [PR 17](https://github.com/cat2151/mascot-render-server/pull/17#pullrequestreview-3988754980)

## Assumptions
- This is an application for personal use, not intended for others. If you desire similar functionality, it's recommended to clone or build your own.
- Destructive changes are frequent. Even if someone were to build related features, they might become unusable the next day.

## Goals of This Application
- PoC. Demonstrate (and demonstrated) that it's possible to create a useful personal application with Codex CLI (Codex Plus 30-day free trial).
- PSD. Easily handle PSDs with Rust.
- Desktop Mascot. Easily implement a desktop mascot with Rust.
- Eye blinking and lip-syncing.
- Server. Easily control the desktop mascot from other applications via HTTP REST API.

## Out of Scope
- Formulation of a new high-functional general-purpose desktop mascot common standard, establishment of a governance system for it, and continuous operation.
- Support. Responding to requests or suggestions.
