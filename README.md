# mascot-render-server

A simplified desktop mascot, written in Rust.

## Features
- Easy installation. Just place the zip file.
- Easy editing. Intuitively change outfits and poses using TUI.
- Fun reactions. Click on the head and...
- This is a simplified version. It has limited features, so a playful spirit is required.

## install

Rust is required.

```
cargo install --force --git https://github.com/cat2151/mascot-render-server mascot-render-server psd-viewer-tui
```

## Setup

Please place the following three zip files in `C:/Users/<YOUR NAME>/AppData/Local/mascot-render-server/assets/zip/`.

Created by Sakamoto Ahiru:
- `ずんだもん立ち絵素材2.3.zip`
- `ずんだもん立ち絵素材V3.2.zip`
- `ずんだもん立ち絵素材改1.1.1.zip`

## Run

```
psd-viewer-tui
```

- The following will be performed automatically:
    - Unzipping of files
    - Analysis of PSD files within the zip
    - Analysis of layers within the PSD files
    - Display of the desktop mascot
        - Displayed with default layers

- Changing outfit or pose layers will alter the appearance.
- If you register favorites in the TUI, the server will refer to the cache and shuffle playback to a different favorite every minute.

- For detailed features, please refer to the on-screen help.

## Configuration
- mascot-render-server.toml
    - always_bouncing
        - If set to `true`, a small squash/stretch breathing-like motion is continuously played for UX verification.
    - always_bend
        - If set to `true`, the single mascot image is rendered with a gentle mesh bend so the upper side sways left and right more than the lower side.
        - While this mode is active, transparent click-through and head-hitbox click detection are disabled so pointer interaction does not drift away from the bent mesh.
    - always_idle_sink
        - IdleSink breathing setting exclusively for `always_bouncing`.
        - By default, it sinks and rises more gently than regular `squash_bounce`, and the tempo subtly changes in sync with the median fluctuation of the blink interval.
        - `sink_amount` and `lift_amount` allow individual adjustment of the exhalation and inhalation poses.
    - transparent_background_click_through
        - If set to `true`, it becomes very heavy, but in return, it reduces "confusion from dragging empty space."
    - flash_blue_background_on_transparent_input
        - If set to `true`, when attempting to click or drag empty space, a blue background flashes for 1 second to notify you.

## Architecture
- Modular
    - Implemented by dividing into small crates based on responsibilities, with an emphasis on reusability.
- Server
    - Dedicated to being a visual-render-server. The idea is to offload responsibilities such as dialogue and motion orchestration to a separate, higher-level application.
- Sixel
    - For fallback in case the desktop mascot fails to run, a preview of the mascot is displayed in the terminal.
- PSDTool
    - Supports extended formats of [PSDTool](https://oov.github.io/psdtool/manual.html), "radio buttonization" and "forced display," to enable comfortable editing.
- Format
    - Management formats like "ghost" or "shell" have not been implemented.
        - Currently, only tested with Zundamon standing pose materials created by Sakamoto Ahiru.
        - It might be possible to use it generically by rewriting the toml file, but this is unconfirmed.

## About `vendor/`

- `vendor/rawpsd` is a vendored copy with AI-assisted fixes for bugs in the [rawpsd](https://github.com/wareya/rawpsd-rs) library.
- It used to panic when trying to read PSDs handled by mascot-render-server, so Codex CLI was used to fix that.
- Refactored due to CI considerations. [PR 17](https://github.com/cat2151/mascot-render-server/pull/17#pullrequestreview-3988754980)

## Assumptions
- This is an application for personal use and is not intended for others. If you desire similar functionality, we recommend cloning or creating your own.
- Destructive changes are made frequently. Even if someone were to build related functionality, it might become unusable the next day.

## Goals of this Application
- PoC (Proof of Concept). To demonstrate (and has demonstrated) that useful applications for personal use can be created with Codex CLI (Codex Plus 30-day free trial).
- PSD. Easily handle PSDs with Rust.
- Desktop Mascot. Easily implement a desktop mascot with Rust.
- Eye blinking and mouth movements.
- Server. Easily control the desktop mascot from other applications via HTTP REST API.

## What this Application Does Not Aim For (Out of Scope)
- Establishing new high-functionality general-purpose desktop mascot common standards, developing governance systems for them, and continuous operation.
- Support. Responding to requests or suggestions.
