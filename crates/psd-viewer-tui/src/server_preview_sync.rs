use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread;

use anyhow::Result;
use mascot_render_control::sync_mascot_render_server_preview;
use mascot_render_protocol::PreviewTargetRequest;

const SERVER_SYNC_ACTIVITY_MESSAGE: &str = "Starting mascot-render-server / syncing preview...";

#[derive(Debug, Default)]
pub(crate) struct ServerPreviewSyncState {
    desired_target: Option<PreviewTargetRequest>,
    active_target: Option<PreviewTargetRequest>,
    synced_target: Option<PreviewTargetRequest>,
}

#[derive(Debug)]
struct ServerPreviewSyncEvent {
    generation: u64,
    target: PreviewTargetRequest,
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

    pub(crate) fn request(&mut self, target: Option<PreviewTargetRequest>) {
        if let Some(next_target) = self.state.request(target) {
            self.spawn_sync(next_target);
        }
    }

    pub(crate) fn drain_completions(&mut self) -> Option<anyhow::Error> {
        loop {
            match self.result_rx.try_recv() {
                Ok(event) if event.generation != self.generation => continue,
                Ok(event) => match event.result {
                    Ok(()) => {
                        if let Some(next_target) = self.state.finish_success(event.target) {
                            self.spawn_sync(next_target);
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

    fn spawn_sync(&self, target: PreviewTargetRequest) {
        let result_tx = self.result_tx.clone();
        let generation = self.generation;
        thread::spawn(move || {
            let result = sync_mascot_render_server_preview(
                &mascot_render_core::mascot_config_path(),
                Some(&target),
            );
            let _ = result_tx.send(ServerPreviewSyncEvent {
                generation,
                target,
                result,
            });
        });
    }

    fn drain_stale_events(&mut self) {
        while self.result_rx.try_recv().is_ok() {}
    }
}

impl ServerPreviewSyncState {
    pub(crate) fn request(
        &mut self,
        target: Option<PreviewTargetRequest>,
    ) -> Option<PreviewTargetRequest> {
        self.desired_target = target;
        self.schedule_next()
    }

    pub(crate) fn finish_success(
        &mut self,
        target: PreviewTargetRequest,
    ) -> Option<PreviewTargetRequest> {
        self.active_target = None;
        self.synced_target = Some(target);
        self.schedule_next()
    }

    pub(crate) fn is_busy(&self) -> bool {
        self.active_target.is_some()
    }

    pub(crate) fn reset(&mut self) {
        self.desired_target = None;
        self.active_target = None;
        self.synced_target = None;
    }

    fn schedule_next(&mut self) -> Option<PreviewTargetRequest> {
        if self.active_target.is_some() {
            return None;
        }

        let Some(next_target) = self.desired_target.clone() else {
            self.synced_target = None;
            return None;
        };

        if self.synced_target.as_ref() == Some(&next_target) {
            return None;
        }

        self.active_target = Some(next_target.clone());
        Some(next_target)
    }
}

#[cfg(test)]
impl ServerPreviewSyncState {
    pub(crate) fn active_png_path_for_test(&self) -> Option<&std::path::Path> {
        self.active_target
            .as_ref()
            .map(|target| target.png_path.as_path())
    }

    pub(crate) fn synced_png_path_for_test(&self) -> Option<&std::path::Path> {
        self.synced_target
            .as_ref()
            .map(|target| target.png_path.as_path())
    }
}
