# mascot-render-server

A desktop mascot (simplified version), written in Rust.

## Features
- Easy installation: just place a zip file.
- Effortless editing: intuitively change outfits and poses using a TUI.
- Amusing reactions: click its head and see...
- This is a simplified version. It has limited features, so a playful spirit is required.

## Install

Rust is required.

```
cargo install --force --git https://github.com/cat2151/mascot-render-server mascot-render-server psd-viewer-tui
```

## Preparation

Please place the following three zip files in `C:/Users/<YOUR NAME>/AppData/Local/mascot-render-server/assets/zip/`.

Created by Ahiru Sakamoto:
- `ずんだもん立ち絵素材2.3.zip`
- `ずんだもん立ち絵素材V3.2.zip`
- `ずんだもん立ち絵素材改1.1.1.zip`

## Execution

```
psd-viewer-tui
```

- The following will be performed automatically:
    - Unpacking zip files
    - Analyzing PSD files contained in the zips
    - Analyzing layers within the PSDs
    - Displaying the desktop mascot
        - It will be displayed with default layers.

- Changing outfit or pose layers will alter its appearance.

- For detailed features, please refer to the on-screen help.

## Configuration
- mascot-render-server.toml
    - transparent_background_click_through
        - If set to true, it will become very heavy, but it reduces "confusion from dragging empty space".
    - flash_blue_background_on_transparent_input
        - If set to true, when you try to click or drag empty space, it will flash a blue background for 1 second to notify you.

## Architecture
- Modular
    - Emphasizes reusability, implemented by dividing responsibilities into small crates.
- Sixel
    - For fallback in case the desktop mascot fails to run, a preview of the mascot will be displayed in the terminal.
- PSDTool
    - Supports [PSDTool](https://oov.github.io/psdtool/manual.html)'s extended formats "radio button conversion" and "forced display", enabling comfortable editing.
- Format
    - Management formats like ghost or shell have not been implemented.
        - Currently, testing has only been done with Zundamon standing pose materials created by Ahiru Sakamoto.
        - There is a possibility that it can be used generically by rewriting the toml file, but this is unconfirmed.

## About vendor/

- `vendor/rawpsd` is a vendored copy of the [rawpsd](https://github.com/wareya/rawpsd-rs) library, with AI-assisted fixes for specific bugs.
- It was causing a panic when trying to read PSDs handled by mascot-render-server, so Codex CLI was used to fix that.

## Assumptions
- This is an application for personal use, and is not intended for others. If you want similar functionality, we recommend cloning or creating your own.
- Destructive changes will be made frequently. Even if someone builds related functionality, it might become unusable the next day.

## Goals of this Application
- PoC: To demonstrate that a useful personal application can be built with Codex CLI (Codex Plus 30-day free trial) (demonstrated).
- PSD: To easily handle PSDs in Rust.
- Desktop Mascot: To easily implement a desktop mascot in Rust.
- Eye blinking and mouth movements.
- Server: To easily control the desktop mascot from other applications via an HTTP REST API.

## Non-Goals (Out of Scope)
- Establishing new highly functional, generic desktop mascot common standards, developing governance systems for them, and continuous operation.
- Support: Responding to requests or suggestions.