use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread;

use anyhow::Result;
use mascot_render_server::sync_mascot_render_server_preview;
use mascot_render_core::mascot_config_path;

const SERVER_SYNC_ACTIVITY_MESSAGE: &str = "Starting mascot-render-server / syncing preview...";

#[derive(Debug, Default)]
pub(crate) struct ServerPreviewSyncState {
    desired_png_path: Option<PathBuf>,
    active_png_path: Option<PathBuf>,
    synced_png_path: Option<PathBuf>,
}

#[derive(Debug)]
struct ServerPreviewSyncEvent {
    generation: u64,
    png_path: PathBuf,
    result: Result<()>,
}

#[derive(Debug)]
pub(crate) struct ServerPreviewSync {
    state: ServerPreviewSyncState,
    generation: u64,
    result_tx: Sender<ServerPreviewSyncEvent>,
    result_rx: Receiver<ServerPreviewSyncEvent>,
}

impl ServerPreviewSync {
    pub(crate) fn new() -> Self {
        let (result_tx, result_rx) = mpsc::channel();
        Self {
            state: ServerPreviewSyncState::default(),
            generation: 0,
            result_tx,
            result_rx,
        }
    }

    pub(crate) fn request(&mut self, png_path: Option<&Path>) {
        if let Some(next_png_path) = self.state.request(png_path) {
            self.spawn_sync(next_png_path);
        }
    }

    pub(crate) fn drain_completions(&mut self) -> Option<anyhow::Error> {
        loop {
            match self.result_rx.try_recv() {
                Ok(event) if event.generation != self.generation => continue,
                Ok(event) => match event.result {
                    Ok(()) => {
                        if let Some(next_png_path) = self.state.finish_success(event.png_path) {
                            self.spawn_sync(next_png_path);
                        }
                    }
                    Err(error) => {
                        self.cancel();
                        return Some(error);
                    }
                },
                Err(TryRecvError::Empty) | Err(TryRecvError::Disconnected) => return None,
            }
        }
    }

    pub(crate) fn activity_message(&self) -> Option<&'static str> {
        self.state.is_busy().then_some(SERVER_SYNC_ACTIVITY_MESSAGE)
    }

    pub(crate) fn cancel(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.state.reset();
        self.drain_stale_events();
    }

    fn spawn_sync(&self, png_path: PathBuf) {
        let result_tx = self.result_tx.clone();
        let generation = self.generation;
        thread::spawn(move || {
            let result =
                sync_mascot_render_server_preview(&mascot_config_path(), Some(png_path.as_path()));
            let _ = result_tx.send(ServerPreviewSyncEvent {
                generation,
                png_path,
                result,
            });
        });
    }

    fn drain_stale_events(&mut self) {
        while self.result_rx.try_recv().is_ok() {}
    }
}

impl ServerPreviewSyncState {
    pub(crate) fn request(&mut self, png_path: Option<&Path>) -> Option<PathBuf> {
        self.desired_png_path = png_path.map(Path::to_path_buf);
        self.schedule_next()
    }

    pub(crate) fn finish_success(&mut self, png_path: PathBuf) -> Option<PathBuf> {
        self.active_png_path = None;
        self.synced_png_path = Some(png_path);
        self.schedule_next()
    }

    pub(crate) fn is_busy(&self) -> bool {
        self.active_png_path.is_some()
    }

    pub(crate) fn reset(&mut self) {
        self.desired_png_path = None;
        self.active_png_path = None;
        self.synced_png_path = None;
    }

    fn schedule_next(&mut self) -> Option<PathBuf> {
        if self.active_png_path.is_some() {
            return None;
        }

        let Some(next_png_path) = self.desired_png_path.clone() else {
            self.synced_png_path = None;
            return None;
        };

        if self.synced_png_path.as_deref() == Some(next_png_path.as_path()) {
            return None;
        }

        self.active_png_path = Some(next_png_path.clone());
        Some(next_png_path)
    }
}

#[cfg(test)]
impl ServerPreviewSyncState {
    pub(crate) fn active_png_path_for_test(&self) -> Option<&Path> {
        self.active_png_path.as_deref()
    }

    pub(crate) fn synced_png_path_for_test(&self) -> Option<&Path> {
        self.synced_png_path.as_deref()
    }
}

