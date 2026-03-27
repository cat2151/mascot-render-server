# mascot-render-server

Desktop mascot (simplified version). Written in Rust.

## Features
- Easy installation. Just place the zip file.
- Effortless editing. Intuitively change outfits and poses using the TUI.
- Amusing reactions. Click the head and...
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
    - Extraction of zip files
    - Analysis of PSD files within the zip
    - Analysis of layers within the PSD
    - Display of the desktop mascot
        - Displayed with default layers

- Changing outfit or pose layers will alter the appearance.
- If you register favorites in the TUI, the server will refer to the cache and shuffle play through different favorites every minute.

- Refer to the on-screen help for detailed features.

## Settings
- mascot-render-server.toml
    - always_bouncing
        - If set to true, a small squash/stretch breathing-like motion will constantly play for UX verification.
    - always_idle_sink
        - This is an IdleSink breathing setting specifically for `always_bouncing`.
        - By default, it sinks and lifts more gently than normal `squash_bounce`, and the tempo changes gradually in sync with the median fluctuation of the blink interval.
        - `sink_amount` and `lift_amount` let you tune the exhale and inhale pose separately.
    - transparent_background_click_through
        - If set to true, it becomes very heavy, but you can reduce "confusion from dragging empty space."
    - flash_blue_background_on_transparent_input
        - If set to true, when you try to click or drag empty space, it will display a blue background for 1 second to notify you.

## Architecture
- Modular
    - Implemented with an emphasis on reusability, split into small crates for each responsibility.
- Server
    - Dedicated to being a visual render server. Responsibilities such as orchestrating dialogue and motion are intended to be handled by a separate, higher-level application.
- Sixel
    - As a fallback in case the desktop mascot doesn't run, a preview of the mascot will be displayed in the terminal.
- PSDTool
    - Supports [PSDTool](https://oov.github.io/psdtool/manual.html)'s extended formats "radio button conversion" and "forced display conversion" to enable comfortable editing.
- Format
    - Management formats like ghost or shell have not been implemented.
        - Currently, it has only been tested with Zundamon standing pose material created by Ahiru Sakamoto.
        - It might be possible to use it generically by rewriting the toml file, but this is unconfirmed.

## About vendor/

- `vendor/rawpsd` is a vendored copy with AI-assisted fixes for bugs in the [rawpsd](https://github.com/wareya/rawpsd-rs) library.
- It used to panic when trying to read PSDs handled by `mascot-render-server`, so Codex CLI was used to fix that.
- Refactored due to CI considerations. [PR 17](https://github.com/cat2151/mascot-render-server/pull/17#pullrequestreview-3988754980)

## Prerequisites
- This is an application for personal use and is not intended for others. If you want similar functionality, we recommend cloning or creating your own.
- Destructive changes will be made frequently. Even if someone builds related functionality, it might become unusable the next day.

## Goals of this application
- PoC. To demonstrate (and has demonstrated) that useful personal applications can be built with Codex CLI (Codex Plus 30-day free trial).
- PSD. Easily handle PSDs in Rust.
- Desktop Mascot. Easily implement a desktop mascot in Rust.
- Eye blinking and lip-syncing.
- Server. Easily control the desktop mascot from other applications via HTTP REST API.

## Out of scope
- Development of new high-functional general-purpose desktop mascot common standards, establishment of a governance system for it, and continuous operation.
- Support. Responding to requests and suggestions.
