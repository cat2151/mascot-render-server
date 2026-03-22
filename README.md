# mascot-render-server

Desktop mascot (simplified version). Written in Rust.

## Features
- Easy installation. Just place the zip file.
- Effortless editing. Intuitively change outfits and poses using the TUI.
- Amusing reactions. Click the head and...
- This is a simplified version. It has limited features, so a playful spirit is required.

## install

```
cargo install --force --git https://github.com/cat2151/mascot-render-server mascot-render-server psd-viewer-tui
```

## Preparation

Rust is required.

Please place the following three zip files in `C:/Users/<YOUR NAME>/AppData/Local/mascot-render-server/assets/zip/`.

By Sakamoto Ahiru:
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
        - Displayed with default layers

- Changing outfit or pose layers will alter its appearance.

- Refer to the on-screen help for detailed features.

## Configuration
- mascot-render-server.toml
    - transparent_background_click_through
        - If set to true, it can significantly reduce "confusion caused by dragging empty space," but at the cost of being very heavy.
    - flash_blue_background_on_transparent_input
        - If set to true, it will flash a blue background for 1 second to notify you when you try to click or drag empty space.

## Architecture
- Modular
    - Prioritizing reusability, it's implemented by dividing into small crates, each responsible for specific tasks.
- Sixel
    - This is a fallback in case the desktop mascot doesn't run, displaying a preview of the mascot in the terminal.
- PSDTool
    - It supports the extended formats of [PSDTool](https://oov.github.io/psdtool/manual.html), "radio button conversion" and "forced display," enabling comfortable editing.
- format
    - Management formats like ghost or shell have not been implemented.
        - Currently, it has only been tested with Zundamon standing pose materials created by Sakamoto Ahiru.
        - While it might be possible to use it generically by rewriting the toml file, this is unconfirmed.

## About `vendor/`

`vendor/rawpsd` is a vendored copy of the [rawpsd](https://github.com/wareya/rawpsd-rs) library, with AI-assisted fixes for bugs.

## Assumptions
- This is a personal application, not intended for use by others. If you desire similar functionality, we recommend cloning or creating your own.
- Destructive changes are made frequently. Even if someone were to build related features, they might become unusable the next day.

## Goals of this application
- PoC. To demonstrate (and has demonstrated) that a useful personal application can be built with Codex CLI (Codex Plus 30-day free trial).
- PSD. To easily handle PSDs in Rust.
- Desktop Mascot. To easily implement a desktop mascot in Rust.
- Blinking eyes and lip-syncing.
- Server. To easily control the desktop mascot from other applications via an HTTP REST API.

## Non-Goals (Out of Scope)
- Development of a new high-functional universal desktop mascot standard, establishment of a governance system for it, and continuous operation.
- Support. Responding to requests and suggestions.