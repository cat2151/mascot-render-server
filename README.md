# mascot-render-server

Desktop mascot (simplified version). Written in Rust.

## Features
- Easy installation. Just place the zip file.
- Effortless editing. Intuitively change outfits and poses using the TUI.
- Fun reactions. Click on the head and...
- This is a simplified version. It has limited features, so a playful spirit is required.

## install

```
cargo install --force --git https://github.com/cat2151/mascot-render-server mascot-render-server psd-viewer-tui
```

## Preparation

Rust is required.

Please place the following three zip files into `C:/Users/<YOUR NAME>/AppData/Local/mascot-render-server/assets/zip/`.

Created by Ahiru Sakamoto
- `ずんだもん立ち絵素材2.3.zip`
- `ずんだもん立ち絵素材V3.2.zip`
- `ずんだもん立ち絵素材改1.1.1.zip`

## Execution

```
psd-viewer-tui
```

- The following actions will be performed automatically:
    - Unzipping the zip files
    - Analyzing the PSD files contained in the zip files
    - Analyzing the layers within the PSD files
    - Displaying the desktop mascot
        - It will be displayed with default layers.

- Changing outfit and pose layers will alter the appearance.

- For detailed features, please refer to the on-screen help.

## Settings
- mascot-render-server.toml
    - transparent_background_click_through
        - If set to true, it becomes very heavy, but reduces "confusion from dragging empty space".
    - flash_blue_background_on_transparent_input
        - If set to true, when you try to click or drag empty space, it will flash a blue background for 1 second to notify you.

## Architecture
- Modular
    - Implemented by emphasizing reusability and splitting into small crates, each with a specific responsibility.
- Sixel
    - For fallback in case the desktop mascot doesn't run, a preview of the mascot will be displayed in the terminal.
- Format
    - Management formats like ghost or shell have not been implemented.
        - Currently, testing has only been done with Zundamon standing pose materials created by Ahiru Sakamoto.
        - It might be possible to use it generically by rewriting the toml file, but this is unconfirmed.

## About vendor/

`vendor/rawpsd` is a vendored copy with AI-assisted fixes applied to a bug in the `rawpsd` library.

## Prerequisites
- This is a personal application, not intended for others to use. If you need similar functionality, cloning or creating your own is recommended.
- Destructive changes are frequent. Even if someone builds related functionality, it might become unusable the very next day.

## Goals of this application
- PoC. To demonstrate that a useful personal application can be built with Codex CLI (Codex Plus 30-day free trial) (demonstrated).
- PSD. To easily handle PSD files in Rust.
- Desktop Mascot. To easily implement a desktop mascot in Rust.
- Eye blinking and lip-syncing.
- Server. To easily control the desktop mascot from other applications via an HTTP REST API.

## Out of scope
- Establishing a new high-functional general-purpose desktop mascot common standard, preparing a governance system for it, and continuous operation.
- Support. Responding to requests and suggestions.