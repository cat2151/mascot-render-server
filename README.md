# mascot-render-server

Desktop mascot (simplified version). Written in Rust.

## Features
- Easy installation. Just place the zip file.
- Effortless editing. Intuitively change outfits and poses using the TUI.
- Playful reactions. Click its head and...
- This is a simplified version. It has limited features, so a playful spirit is required.

## Install

Rust is required.

```
cargo install --force --git https://github.com/cat2151/mascot-render-server mascot-render-server psd-viewer-tui
```

## Preparation

Please place the following three zip files in `C:/Users/<YOUR NAME>/AppData/Local/mascot-render-server/assets/zip/`.

Created by Sakamoto Ahiru
- `ずんだもん立ち絵素材2.3.zip`
- `ずんだもん立ち絵素材V3.2.zip`
- `ずんだもん立ち絵素材改1.1.1.zip`

## Run

```
psd-viewer-tui
```

- The following will happen automatically:
    - Extraction of zip files
    - Analysis of PSD files within zips
    - Analysis of layers within PSDs
    - Display of the desktop mascot
        - Displayed with default layers

- Changing outfit or pose layers will alter its appearance.
- If you register favorites in the TUI, the server will reference the cache and shuffle playback to a different favorite every minute.

- Refer to the on-screen help for detailed features.

## Configuration
- mascot-render-server.toml
    - always_idle_sink
        - If set to `true`, a small IdleSink breath-like motion for UX verification will be constantly played.
    - always_bend
        - If set to `true`, a left/right bend for UX verification will be constantly played.
    - bend
        - With `amplitude_ratio`, you can specify the bend width as a ratio to the mascot image width. The default is `0.0075`.
    - idle_sink
        - This is the IdleSink breathing setting specifically for `always_idle_sink`.
        - By default, it sinks and rises more gently than a regular squash_bounce, and the tempo gradually changes in sync with the fluctuations in the median blink interval.
        - You can individually adjust the exhalation and inhalation poses using `sink_amount` and `lift_amount`.

## Architecture
- Modular
    - Emphasis on reusability, implemented by dividing into small crates based on responsibility.
- Server
    - Dedicated to being a visual render server. The responsibility for orchestrating dialogue and motion, etc., is intended to be handled by a separate higher-level application.
- Sixel
    - As a fallback in case the desktop mascot doesn't run, a preview of the mascot will be displayed in the terminal.
- PSDTool
    - Supports the extended formats of [PSDTool](https://oov.github.io/psdtool/manual.html), 'radio button conversion' and 'forced display', enabling comfortable editing.
- Format
    - Management formats like ghost or shell have not been implemented.
        - Currently, testing has only been done with Zundamon standing pose materials created by Sakamoto Ahiru.
        - It's possible it could be used generically by rewriting the toml file, but this is unconfirmed.

## About `vendor/`

- `vendor/rawpsd` is a vendored copy of the [rawpsd](https://github.com/wareya/rawpsd-rs) library with AI-assisted fixes for bugs.
- It used to panic when trying to read PSDs handled by mascot-render-server, so Codex CLI was used to fix that.
- Refactored due to CI considerations. [PR 17](https://github.com/cat2151/mascot-render-server/pull/17#pullrequestreview-3988754980)

## Premise
- This application is for personal use and is not intended for others. If you want similar functionality, I recommend cloning it or building your own.
- Destructive changes are made frequently. Even if someone builds related functionality, it might become unusable the next day.

## What this application aims for
- PoC. To demonstrate (and has demonstrated) that useful personal applications can be built with Codex CLI (Codex Plus 30-day free trial).
- PSD. Ease of handling PSDs with Rust.
- Desktop Mascot. Ease of realizing desktop mascots with Rust.
- Eye blinking and lip-sync.
- Server. Ease of controlling the desktop mascot from other applications via HTTP REST API.

## What this application does not aim for (out of scope)
- Establishment of new highly functional general-purpose desktop mascot common standards, development of governance systems for them, and continuous operation.
- Support. Responding to requests and suggestions.