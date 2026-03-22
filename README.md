# mascot-render-server

A simplified desktop mascot, written in Rust.

## Features
- Easy installation. Just place the zip file.
- Effortless editing. Intuitively change outfits and poses using the TUI.
- Fun reactions. Click on its head...
- This is a simplified version. It has few features, so a playful spirit is required.

## Install

Rust is required.

```
cargo install --force --git https://github.com/cat2151/mascot-render-server mascot-render-server psd-viewer-tui
```

## Preparation

Please place the following three zip files in `C:/Users/<YOUR NAME>/AppData/Local/mascot-render-server/assets/zip/`:

By Sakamoto Ahiru
- `ずんだもん立ち絵素材2.3.zip`
- `ずんだもん立ち絵素材V3.2.zip`
- `ずんだもん立ち絵素材改1.1.1.zip`

## Execution

```
psd-viewer-tui
```

- The following will happen automatically:
    - Extraction of zip files
    - Analysis of PSD files within the zip files
    - Analysis of layers within the PSD files
    - Display of the desktop mascot
        - It will be displayed with default layers

- Changing outfit and pose layers will alter its appearance.

- For detailed features, please refer to the help screen.

## Settings
- mascot-render-server.toml
    - always_bouncing
        - If set to true, it keeps a small squash/stretch breathing-like motion running continuously for UX experiments.
    - transparent_background_click_through
        - If set to true, it becomes very resource-intensive, but reduces "confusion from dragging empty space".
    - flash_blue_background_on_transparent_input
        - If set to true, when you try to click or drag empty space, it will display a blue background for 1 second to notify you.

## Architecture
- Modular
    - Prioritizing reusability, it's implemented by splitting responsibilities into small crates.
- Server
    - Dedicated to being a visual-render-server. The responsibility for orchestrating dialogue and motion, etc., is intended to be handled by a separate, higher-level application.
- Sixel
    - For fallback in case the desktop mascot doesn't run, a preview of the mascot will be displayed in the terminal.
- PSDTool
    - Supports the extended formats of [PSDTool](https://oov.github.io/psdtool/manual.html) — "radio button conversion" and "forced display" — enabling comfortable editing.
- Format
    - Management formats like ghost or shell have not been implemented.
        - Currently, testing has only been done with Zundamon standing pose materials by Sakamoto Ahiru.
            - It might be possible to use it generically by rewriting the toml file, but this is unconfirmed.

## About vendor/

- `vendor/rawpsd` is a vendored copy of the [rawpsd](https://github.com/wareya/rawpsd-rs) library, with fixes applied with AI assistance to address bugs.
- It used to panic when trying to read PSDs handled by mascot-render-server, so Codex CLI was used to fix that.

## Assumptions
- This application is for personal use, not intended for others. If you need similar functionality, cloning or creating your own is recommended.
- Destructive changes will be made frequently. Even if someone builds related functionality, it might become unusable the next day.

## What this app aims for
- PoC. To demonstrate (and has demonstrated) that useful personal applications can be created with Codex CLI (Codex Plus 30-day free trial).
- PSD. To easily handle PSDs in Rust.
- Desktop Mascot. To easily implement a desktop mascot in Rust.
- Eye blinking and lip-syncing.
- Server. To easily control the desktop mascot from other applications via HTTP REST API.

## What this app does not aim for (out of scope)
- Establishing a new highly functional, general-purpose desktop mascot common standard, developing a governance system for it, and continuous operation.
- Support. Responding to requests or proposals.
