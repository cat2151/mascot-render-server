# mascot-render-server

Desktop mascot (simplified version). Written in Rust.

## Features
- Easy installation. Just place the zip files.
- Effortless editing. Intuitively change outfits and poses using the TUI.
- Playful reactions. Click on the head to...
- This is a simplified version. It has limited features, so a playful spirit is required.

## install

```
cargo install --force --git https://github.com/cat2151/mascot-render-server mascot-render-server psd-viewer-tui
```

## Preparation

Rust is required.

Please place the following three zip files in `C:/Users/<YOUR NAME>/AppData/Local/mascot-render-server/assets/zip/`.

Created by Sakamoto Ahiru:
- `ずんだもん立ち絵素材2.3.zip`
- `ずんだもん立ち絵素材V3.2.zip`
- `ずんだもん立ち絵素材改1.1.1.zip`

## Execution

```
psd-viewer-tui
```

- The following will be performed automatically:
    - Extraction of zip files
    - Analysis of PSD files within the zips
    - Analysis of layers within the PSD files
    - Display of the desktop mascot
        - Displayed with default layers

- Changing outfit and pose layers will alter its appearance.

- Refer to the on-screen help for detailed features.

## Configuration
- mascot-render-server.toml
    - transparent_background_click_through
        - If `true`, it becomes very heavy, but reduces "confusion by dragging empty space".
    - flash_blue_background_on_transparent_input
        - If `true`, attempting to click or drag empty space will briefly show a blue background for 1 second as a notification.

## Architecture
- Modular
    - Emphasizing reusability, it's implemented by dividing it into small crates, each with distinct responsibilities.
- Sixel
    - As a fallback in case the desktop mascot doesn't run, a preview of the mascot is displayed in the terminal.
- PSDTool
    - Supports the extended formats "Radio Button Conversion" and "Forced Display" of [PSDTool](https://oov.github.io/psdtool/manual.html) for comfortable editing.
- format
    - Management formats like ghost or shell are not yet implemented.
        - Currently, it has only been tested with Zundamon standing pose materials created by Sakamoto Ahiru.
        - It might be possible to use it generically by rewriting the toml file, but this is unconfirmed.

## About vendor/

- `vendor/rawpsd` is a vendored copy of the [rawpsd](https://github.com/wareya/rawpsd-rs) library, with AI-assisted fixes for bugs.
- It used to panic when trying to read PSDs handled by `mascot-render-server`, so Codex CLI was used to fix that.

## Prerequisites
- This is an application for personal use, and it's not intended for others. If you want similar functionality, we recommend cloning it or building your own.
- Destructive changes are made frequently. Even if someone builds a related feature, it might become unusable the next day.

## Goals of this application
- PoC. To demonstrate (and has demonstrated) that useful personal applications can be built with Codex CLI (Codex Plus 30-day free trial).
- PSD. To easily handle PSDs in Rust.
- Desktop Mascot. To easily implement a desktop mascot in Rust.
- Eye blinking and lip-syncing.
- Server. To easily control the desktop mascot from other applications via an HTTP REST API.

## Out of scope
- Establishment of a new high-functional general-purpose desktop mascot common standard, development of a governance system for it, and continuous operation.
- Support. Responding to requests and suggestions.