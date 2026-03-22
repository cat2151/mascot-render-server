# mascot-render-server

A desktop mascot (simplified version). Written in Rust.

## Features
- Easy installation. Just place the zip file.
- Easy editing. Intuitively change outfits and poses using the TUI.
- Fun reactions. Clicking its head...
- This is a simplified version. It has limited features, so a playful spirit is required.

## Installation

Rust is required.

```
cargo install --force --git https://github.com/cat2151/mascot-render-server mascot-render-server psd-viewer-tui
```

## Setup

Please place the following three zip files in `C:/Users/<YOUR NAME>/AppData/Local/mascot-render-server/assets/zip/`.

Created by Ahiru Sakamoto
- `ずんだもん立ち絵素材2.3.zip`
- `ずんだもん立ち絵素材V3.2.zip`
- `ずんだもん立ち絵素材改1.1.1.zip`

## Running

```
psd-viewer-tui
```

- The following will be performed automatically:
    - Extraction of zip files
    - Analysis of PSD files within the zip
    - Analysis of layers within the PSD
    - Display of the desktop mascot
        - Displayed with default layers

- Changing outfit and pose layers will alter its appearance.

- For detailed features, refer to the on-screen help.

## Settings
- mascot-render-server.toml
    - always_bouncing
        - If `true`, a small squash/stretch breathing-like motion is constantly played for UX validation.
    - transparent_background_click_through
        - If `true`, it becomes very heavy, but reduces "confusion from dragging empty space."
    - flash_blue_background_on_transparent_input
        - If `true`, when attempting to click or drag empty space, a blue background flashes for 1 second to notify.

## Architecture
- Modular
    - Implemented by splitting into small crates, emphasizing reusability and separation of concerns.
- Server
    - Dedicated to being a visual render server. Responsibilities such as orchestrating dialogue and motion are intended to be handled by a separate, higher-level application.
- Sixel
    - For fallback in case the desktop mascot fails to run, a preview of the mascot is displayed in the terminal.
- PSDTool
    - Supports the extended formats of [PSDTool](https://oov.github.io/psdtool/manual.html), "radio button conversion" and "forced display," enabling comfortable editing.
- Format
    - Management formats like ghost or shell have not been implemented.
        - Currently, it has only been tested with Zundamon standing pose materials created by Ahiru Sakamoto.
        - It might be usable generically by modifying the toml file, but this is unconfirmed.

## About `vendor/`

- `vendor/rawpsd` is a vendored copy that includes AI-assisted fixes for bugs in the [rawpsd](https://github.com/wareya/rawpsd-rs) library.
- It was panicking when trying to read PSDs handled by `mascot-render-server`, so Codex CLI was used to fix that.

## Assumptions
- This application is for personal use and not intended for others. If you desire similar functionality, it is recommended to clone or create your own.
- Destructive changes are made frequently. Should anyone develop related functionality, it might become unusable the very next day.

## Goals of this application
- PoC. To demonstrate (and has demonstrated) that useful personal applications can be created with Codex CLI (Codex Plus 30-day free trial).
- PSD. Easy handling of PSDs in Rust.
- Desktop mascot. Easy implementation of a desktop mascot in Rust.
- Eye blinking and mouth movements.
- Server. Easy control of the desktop mascot from other applications via HTTP REST API.

## What this application does NOT aim for (out of scope)
- Establishing new high-functional general-purpose desktop mascot common standards, setting up a governance system for them, or continuous operation.
- Support. Responding to requests or suggestions.