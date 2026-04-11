use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

use mascot_render_client::mascot_render_server_status;
use mascot_render_protocol::ServerStatusSnapshot;

pub(crate) struct StatusPollSync {
    result_rx: Option<Receiver<Result<ServerStatusSnapshot, String>>>,
}

impl StatusPollSync {
    pub(crate) fn new() -> Self {
        Self { result_rx: None }
    }

    pub(crate) fn start_if_idle(&mut self) -> bool {
        if self.result_rx.is_some() {
            return false;
        }

        let (result_tx, result_rx) = mpsc::channel();
        self.result_rx = Some(result_rx);
        thread::spawn(move || {
            let result = mascot_render_server_status().map_err(|error| format!("{error:#}"));
            let _ = result_tx.send(result);
        });

        true
    }

    pub(crate) fn drain_completion(&mut self) -> Option<Result<ServerStatusSnapshot, String>> {
        let result_rx = self.result_rx.as_ref()?;
        match result_rx.try_recv() {
            Ok(result) => {
                self.result_rx = None;
                Some(result)
            }
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                self.result_rx = None;
                Some(Err(
                    "mascot-render-server status poll worker disconnected".to_string()
                ))
            }
        }
    }
}
