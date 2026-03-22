# mascot-render-server

Desktop mascot (simplified version). Written in Rust.

## Features
- Easy installation. Just place the zip file.
- Easy editing. Intuitively change outfits and poses via TUI.
- Fun reactions. Click the head and...
- This is a simplified version. It has limited features, so a playful spirit is required.

## Install

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

- The following will be performed automatically:
    - Unzipping of files
    - Analysis of PSD files within the zip
    - Analysis of layers within the PSD
    - Display of the desktop mascot
        - It will be displayed with default layers.

- Changing outfit and pose layers will alter its appearance.

- For detailed features, please refer to the on-screen help.

## Settings
- mascot-render-server.toml
    - always_bouncing
        - If `true`, a small squash/stretch breathing-like motion is constantly played for UX verification.
    - transparent_background_click_through
        - If `true`, it becomes very resource-intensive but reduces 'confusion from dragging empty space'.
    - flash_blue_background_on_transparent_input
        - If `true`, when attempting to click or drag empty space, a blue background flashes for 1 second to notify.

## Architecture
- Modular
    - Prioritizing reusability, it's implemented by splitting into small crates, each with a specific responsibility.
- Server
    - Dedicated to being a visual-render-server. Responsibilities such as orchestrating dialogue and motion are intended to be handled by a separate, higher-level application.
- Sixel
    - For fallback in case the desktop mascot doesn't work, a preview of the mascot is displayed in the terminal.
- PSDTool
    - Supports extended formats of [PSDTool](https://oov.github.io/psdtool/manual.html), 'Radio button conversion' and 'Forced display', enabling comfortable editing.
- Format
    - Management formats like ghost or shell have not been implemented.
        - Currently, testing is only performed with Zundamon character portrait materials created by Ahiru Sakamoto.
        - It might be possible to use it generically by rewriting the toml file, but this is unconfirmed.

## About `vendor/`

- `vendor/rawpsd` is a vendored copy of the [rawpsd](https://github.com/wareya/rawpsd-rs) library, with AI-assisted fixes for bugs.
- It was panicking when trying to read PSDs handled by mascot-render-server, so Codex CLI was used to fix that.
- Refactored due to CI considerations. [PR 17](https://github.com/cat2151/mascot-render-server/pull/17#pullrequestreview-3988754980)

## Prerequisites
- This application is for personal use and is not intended for others. If you want similar functionality, we recommend cloning it or building your own.
- Destructive changes are frequent. Even if someone builds a related feature, it might become unusable the next day.

## Goals of this application
- PoC. To demonstrate that useful personal applications can be created with Codex CLI (Codex Plus 30-day free trial) (demonstrated).
- PSD. To easily handle PSDs in Rust.
- Desktop Mascot. To easily implement a desktop mascot in Rust.
- Eye blinking and lip-syncing.
- Server. To easily control the desktop mascot from other applications via HTTP REST API.

## Out of Scope
- Establishment of a new highly functional general-purpose desktop mascot common standard, development of a governance system for it, and continuous operation.
- Support. Responding to requests and suggestions.