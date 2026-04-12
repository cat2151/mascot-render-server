# mascot-render-server

A desktop mascot (simplified version). Written in Rust.

## Features
- Easy installation. Just place the zip file.
- Effortless editing. Intuitively change outfits and poses using the TUI.
- Amusing reactions. Click the head and...
- Speak: [voicevox-playground-tui](https://github.com/cat2151/voicevox-playground-tui/blob/main/README.ja.md)
- All together: Arrange and display standing illustrations of members registered as favorites in the TUI. You can display all of them. (Simplified version)
- This is a simplified version. It has limited features, so a playful spirit is required.

## install

Rust is required.

```
cargo install --force --git https://github.com/cat2151/mascot-render-server mascot-render-server psd-viewer-tui mascot-render-status-tui
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
    - Analysis of PSD files within the zip archives
    - Analysis of layers within the PSD files
    - Display of the desktop mascot
        - Displayed with default layers

- Changing outfit or pose layers will alter its appearance.
- If you register favorites in the TUI, the server will refer to the cache and shuffle play through different favorites every minute.
- For detailed features, please refer to the on-screen help.

## Configuration
- mascot-render-server.toml
    - `always_idle_sink`
        - The initial value is `true`.
        - If set to `true`, a small IdleSink breathing-like motion will be continuously played for UX verification.
    - `always_bend`
        - The initial value is `true`.
        - If set to `true`, continuous left-right bending will be played for UX verification.
    - `bend`
        - The `amplitude_ratio` allows you to specify the bend width as a ratio to the mascot image width. The default is `0.0075`.
    - `idle_sink`
        - This is the IdleSink breathing setting specifically for `always_idle_sink`.
        - By default, it sinks and lifts more gently than a normal squash_bounce, and the tempo changes gradually in sync with fluctuations in the median blink interval.
        - With `sink_amount` and `lift_amount`, you can individually adjust the exhalation and inhalation poses.

## Architecture
- Modular
    - Emphasizing reusability, it is implemented by dividing it into small crates, each responsible for specific tasks.
- Server
    - Dedicated to being a visual-render-server. The idea is to delegate responsibilities like orchestrating dialogue and motion to a separate, higher-level application.
- Sixel
    - For fallback in case the desktop mascot fails to run, a preview of the mascot will be displayed in the terminal.
- PSDTool
    - It supports the extended formats "radio buttonization" and "forced display" of [PSDTool](https://oov.github.io/psdtool/manual.html), enabling comfortable editing.
- Format
    - Management formats like ghost or shell have not been implemented.
        - Currently, it has only been tested with Zundamon Tachi-e Material created by Ahiru Sakamoto.
            - It might be possible to use it generically by rewriting the toml file, but this is unconfirmed.

## About vendor/

- `vendor/rawpsd` is a vendored copy with AI-assisted fixes for bugs in the [rawpsd](https://github.com/wareya/rawpsd-rs) library.
- It was panicking when trying to read PSDs handled by mascot-render-server, so Codex CLI was used to fix that.
- Refactored due to CI considerations. [PR 17](https://github.com/cat2151/mascot-render-server/pull/17#pullrequestreview-3988754980)

## Assumptions
- This is an application for personal use, so it is not intended for others to use. If you desire similar functionality, we recommend cloning or creating your own.
- Destructive changes are frequent. Even if someone builds related functionality, it might become unusable the very next day.

## Goals of this application
- PoC: To demonstrate that a useful application for personal use can be created with Codex CLI (Codex Plus 30-day free trial) (demonstrated).
- PSD: To easily handle PSDs in Rust.
- Desktop Mascot: To easily implement a desktop mascot in Rust.
- Eye blinking and lip-syncing.
- Server: To easily control the desktop mascot from other applications via an HTTP REST API.

## What is NOT aimed for (Out of scope)
- Establishing new high-functionality general-purpose desktop mascot common standards, setting up a governance structure for them, and continuous operation.
- Support: Responding to requests or proposals.