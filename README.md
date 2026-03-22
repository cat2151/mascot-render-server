# mascot-render-server

A simplified desktop mascot, written in Rust.

## Features
- Easy installation. Just place the zip file.
- Easy editing. Intuitively change outfits and poses using the TUI.
- Amusing reactions. Click the head to...
- This is a simplified version. It has limited features, so a playful spirit is required.

## Install

```
cargo install --force --git https://github.com/cat2151/mascot-render-server mascot-render-server psd-viewer-tui
```

## Setup

Rust is required.

Please place the following three zip files in `C:/Users/<YOUR NAME>/AppData/Local/mascot-render-server/assets/zip/`.

By Sakamoto Ahiru:
- `ずんだもん立ち絵素材2.3.zip`
- `ずんだもん立ち絵素材V3.2.zip`
- `ずんだもん立ち絵素材改1.1.1.zip`

## Run

```
psd-viewer-tui
```

- The following will be performed automatically:
    - Zip file extraction
    - Analysis of PSD files contained in the zip
    - Analysis of layers within the PSD
    - Display of the desktop mascot
        - It will be displayed with default layers.

- Changing outfit or pose layers will alter its appearance.

- For detailed features, please refer to the on-screen help.

## Configuration
- mascot-render-server.toml
    - transparent_background_click_through
        - If set to `true`, it will become very resource-intensive, but it can reduce the "confusion from dragging empty space".
    - flash_blue_background_on_transparent_input
        - If set to `true`, when you attempt to click or drag empty space, it will flash a blue background for 1 second to notify you.

## Architecture
- Modular
    - Implemented by dividing into small crates, prioritizing reusability and separating responsibilities.
- Sixel
    - For fallback in case the desktop mascot fails to run, a preview of the mascot will be displayed in the terminal.
- PSDTool
    - Supports the extended formats "radio button conversion" and "forced display" of [PSDTool](https://oov.github.io/psdtool/manual.html), achieving comfortable editing.
- format
    - Management formats like ghost or shell are not implemented.
        - Currently, it has only been tested with Zundamon character portrait materials created by Sakamoto Ahiru.
            - It might be possible to use it generically by rewriting the toml file, but this is unconfirmed.

## About `vendor/`

`vendor/rawpsd` is a vendored copy that includes AI-assisted fixes for bugs in the `rawpsd` library.

## Assumptions
- This is an application for personal use, not intended for others. If you desire similar functionality, it is recommended to clone or create your own.
- Destructive changes are made frequently. Even if someone were to build related functionality, it might become unusable the next day.

## Goals of this application
- PoC. To demonstrate that useful applications can be created for personal use with Codex CLI (Codex Plus 30-day free trial) (demonstrated).
- PSD. Easy handling of PSD files with Rust.
- Desktop Mascot. Easy implementation of desktop mascots with Rust.
- Eye blinking and lip-syncing.
- Server. Easy manipulation of the desktop mascot from other applications via HTTP REST API.

## Out of Scope
- Establishment of a new high-functionality general-purpose desktop mascot common standard, development of a governance system for it, and continuous operation.
- Support. Responding to requests or suggestions.