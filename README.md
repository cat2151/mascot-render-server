# mascot-render-server

A simplified desktop mascot, written in Rust.

## Features
- Easy installation: Just place the zip file.
- Easy editing: Intuitively change outfits and poses using the TUI.
- Fun reactions: Clicking on the head...
- Speaks: [voicevox-playground-tui](https://github.com/cat2151/voicevox-playground-tui/blob/main/README.ja.md)
- Gather everyone: Display and play with character sprites of members registered as favorites in the TUI. You can even display all of them. (Simplified version)
- This is a simplified version. It has limited features, so a playful spirit is required.

## Install

Rust is required.

```
cargo install --force --git https://github.com/cat2151/mascot-render-server mascot-render-server psd-viewer-tui mascot-render-status-tui
```

## Setup

Please place the following three zip files in `C:/Users/<YOUR NAME>/AppData/Local/mascot-render-server/assets/zip/`.

By Ahiru Sakamoto-sama
- `ずんだもん立ち絵素材2.3.zip`
- `ずんだもん立ち絵素材V3.2.zip`
- `ずんだもん立ち絵素材改1.1.1.zip`

## Run

```
psd-viewer-tui
```

- The following will be performed automatically:
    - Unpacking zip files
    - Analyzing PSD files contained in the zip
    - Analyzing layers within the PSD
    - Displaying the desktop mascot
        - Displayed with default layers

- Changing outfit or pose layers will alter the appearance.
- If you register favorites in the TUI, the server will refer to the cache and shuffle playback to a different favorite every minute.

- If you turn on the `Zundamon Performance Link` option in the `cat-smf-editor` settings (`Ctrl+, → b` / Palette), the displayed Zundamon will play a performance-specific squash-bounce animation (`musical_squash_bounce` setting; default: 500ms duration, 3px vertical movement, 1 cycle, 2 damping, 0.1 horizontal, 0.05 vertical) based on its regular click reaction for each quarter note of the performance, and switch mouth images at the default 4 fps during the target vocal note sections. The length of the squash-bounce remains constant, and any ongoing movement is cut short and restarted at the next beat (every 500ms at 120 BPM). It follows tempo changes, mid-playback, and loops. Mouth images use existing delayed generation, and the editor's performance continues during preparation. The ON/OFF state of the linkage is saved by the editor and restored on restart. Stopping, turning OFF, closing the editor, or losing communication will release the server's performance state, but the mascot's appearance, position, and server settings will not be re-saved. SMPTE division beat synchronization is not supported.

- For detailed features, please refer to the on-screen help.

## Configuration
- mascot-render-server.toml
    - always_idle_sink
        - Default value is `true`.
        - If set to `true`, a small IdleSink breathing-like motion is constantly played for UX verification.
    - always_bend
        - Default value is `true`.
        - If set to `true`, a left/right bend motion for UX verification is constantly played.
    - bend
        - The `amplitude_ratio` allows you to specify the bend width as a ratio to the mascot image width. Default is `0.0075`.
    - musical_squash_bounce
        - This is a squash-bounce animation specifically for performance linkage, independent of `squash_bounce` (for regular clicks).
        - You can directly edit `duration_ms` (movement length), `amplitude_px` (vertical movement amount), `cycles` (number of cycles), `damping` (damping), `squash_amount` (horizontal expansion), and `stretch_amount` (vertical compression). Currently, there is only one `algorithm`: `squash_stretch`.
        - Saving the TOML automatically reloads it. Since playback restarts from the beginning of each beat, if `duration_ms` is longer than the beat interval, the movement will be cut short.
    - idle_sink
        - This is the IdleSink breathing setting specifically for `always_idle_sink`.
        - By default, it sinks and lifts more gently than a regular squash_bounce, and the tempo gradually changes in sync with the fluctuations of the median blink interval.
        - You can individually adjust the exhalation and inhalation poses using `sink_amount` and `lift_amount`.

## Architecture
- Modular
    - Implemented with an emphasis on reusability, divided into small crates, each responsible for specific tasks.
- server
    - The `POST /playback` endpoint on localhost maintains the latest performance position, beat tempo, and vocalization enabled state, which is cleared after a 750ms update expiry. The mouth opening/closing cycle is not reset by continuous state updates. This is a separate temporary state from the existing single-shot `/timeline` and does not change the save format.
    - Dedicated to being a visual-render-server. The responsibility for orchestrating dialogue and motion, etc., is intended to be handled by a separate, higher-level application.
- Sixel
    - For fallback in case the desktop mascot doesn't work, a preview of the mascot is displayed in the terminal.
- PSDTool
    - Supports [PSDTool](https://oov.github.io/psdtool/manual.html)'s extended formats 'Radio Buttonization' and 'Forced Display', enabling comfortable editing.
- format
    - Management formats like ghost or shell have not been implemented.
        - Currently, only tested with Zundamon character sprite materials created by Ahiru Sakamoto-sama.
        - There's a possibility it could be used generically by rewriting the TOML file, but this is unconfirmed.

## About `vendor/`

- `vendor/rawpsd` is a vendored copy of the [rawpsd](https://github.com/wareya/rawpsd-rs) library, with AI-assisted fixes for bugs.
- It panicked when attempting to read PSDs handled by mascot-render-server, so Codex CLI was used to fix it.
- Refactored due to CI considerations. [PR 17](https://github.com/cat2151/mascot-render-server/pull/17#pullrequestreview-3988754980)

## Assumptions
- This application is for personal use and not intended for others. If you want similar functionality, we recommend cloning or creating your own.
- Breaking changes will be made frequently. Even if someone builds related functionality, it might become unusable the next day.

## What this application aims for
- PoC: To demonstrate that a useful personal application can be created with Codex CLI (Codex Plus 30-day free trial) (demonstrated).
- PSD: Easy handling of PSDs with Rust.
- Desktop Mascot: Easy implementation of desktop mascots with Rust.
- Blinking and lip-sync.
- Server: Easy manipulation of the desktop mascot from other applications via an HTTP REST API.

## What it does NOT aim for (out of scope)
- Establishing a new high-functionality general-purpose desktop mascot common standard, developing a governance system for it, and continuous operation.
- Support: Responding to requests or suggestions.