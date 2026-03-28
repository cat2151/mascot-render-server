# mascot-render-server

A simplified desktop mascot. Written in Rust.

## Features
- Easy installation. Just place the zip file.
- Effortless editing. Intuitively change outfits and poses using the TUI.
- Amusing reactions. Click its head and...
- This is a simplified version. It has fewer features, so a playful mindset is required.

## Installation

Rust is required.

```
cargo install --force --git https://github.com/cat2151/mascot-render-server mascot-render-server psd-viewer-tui
```

## Setup

Please place the following three zip files in `C:/Users/<YOUR NAME>/AppData/Local/mascot-render-server/assets/zip/`.

Created by SAKAMOTO Ahiru:
- `ずんだもん立ち絵素材2.3.zip`
- `ずんだもん立ち絵素材V3.2.zip`
- `ずんだもん立ち絵素材改1.1.1.zip`

## Running

```
psd-viewer-tui
```

- The following will be performed automatically:
    - Unpacking zip files
    - Analyzing PSD files contained in the zip
    - Analyzing layers within the PSD
    - Displaying the desktop mascot
        - It will be displayed with default layers.

- Changing outfit or pose layers will alter its appearance.
- If you register favorites in the TUI, the server will refer to the cache and shuffle playback to a different favorite every minute.

- For detailed features, please refer to the on-screen help.

## Configuration
- mascot-render-server.toml
    - always_idle_sink
        - If set to true, a small IdleSink breathing-like motion will constantly play for UX verification.
    - always_bend
        - If set to true, the constant side-to-side bend used for UX verification will play.
    - bend
        - `amplitude_ratio` controls bend width as a ratio of the mascot image width. The default is `0.015`.
    - idle_sink
        - This is an IdleSink breathing setting specifically for `always_idle_sink`.
        - By default, it sinks and lifts more gently than a regular `squash_bounce`, and its tempo gradually changes in accordance with the fluctuation of the median blink interval.
        - You can individually adjust the exhalation and inhalation poses using `sink_amount` and `lift_amount`.
    - transparent_background_click_through
        - If set to true, it becomes very resource-intensive, but it reduces the "confusion from dragging empty space."
    - flash_blue_background_on_transparent_input
        - If set to true, when you try to click or drag empty space, it will display a blue background for 1 second to notify you.

## Architecture
- Modular
    - Emphasizes reusability, implemented by dividing into small crates each responsible for a specific concern.
- server
    - Dedicated to being a visual render server. The orchestration of dialogue and motion, and similar responsibilities, are intended to be handled by separate higher-level applications.
- Sixel
    - For fallback in case the desktop mascot fails to run, a preview of the mascot will be displayed in the terminal.
- PSDTool
    - Supports the extended formats of [PSDTool](https://oov.github.io/psdtool/manual.html), "radio buttonization" and "forced display," to enable comfortable editing.
- format
    - Management formats such as ghost or shell are not yet implemented.
        - Currently, testing is only done with Zundamon standing pose materials created by SAKAMOTO Ahiru.
        - It might be possible to use it generically by rewriting the toml file, but this is unconfirmed.

## About `vendor/`

- `vendor/rawpsd` is a vendored copy that includes AI-assisted fixes for bugs in the [rawpsd](https://github.com/wareya/rawpsd-rs) library.
- It used to panic when trying to read PSDs handled by mascot-render-server, so Codex CLI was used to fix that.
- Refactored due to CI considerations. [PR 17](https://github.com/cat2151/mascot-render-server/pull/17#pullrequestreview-3988754980)

## Assumptions
- This application is for my personal use and is not intended for others. If you need similar functionality, I recommend cloning it or building your own.
- Destructive changes will be made frequently. Even if someone builds related functionality, it might become unusable the very next day.

## Goals of this application
- PoC: To demonstrate (and has demonstrated) that a useful personal application can be built with Codex CLI (Codex Plus 30-day free trial).
- PSD: To easily handle PSDs in Rust.
- Desktop Mascot: To easily realize a desktop mascot in Rust.
- Eye blinking and mouth movements.
- Server: To easily control the desktop mascot from other applications via an HTTP REST API.

## Out of Scope
- Establishing new highly-functional, general-purpose desktop mascot common standards, developing a governance system for them, and continuous operation.
- Support: Responding to requests or proposals.
