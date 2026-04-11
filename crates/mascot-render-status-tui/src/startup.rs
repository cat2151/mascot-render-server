use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

use mascot_render_control::ensure_mascot_render_server_running;

pub(crate) struct ServerStartupSync {
    config_path: PathBuf,
    result_rx: Option<Receiver<Result<(), String>>>,
    attempted: bool,
}

impl ServerStartupSync {
    pub(crate) fn new(config_path: PathBuf) -> Self {
        Self {
            config_path,
            result_rx: None,
            attempted: false,
        }
    }

    pub(crate) fn start_if_idle(&mut self) -> bool {
        if self.attempted || self.result_rx.is_some() {
            return false;
        }

        let config_path = self.config_path.clone();
        let (result_tx, result_rx) = mpsc::channel();
        self.result_rx = Some(result_rx);
        self.attempted = true;

        thread::spawn(move || {
            let result = ensure_mascot_render_server_running(&config_path)
                .map_err(|error| format!("{error:#}"));
            let _ = result_tx.send(result);
        });

        true
    }

    pub(crate) fn drain_completion(&mut self) -> Option<Result<(), String>> {
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
                    "mascot-render-server startup worker disconnected".to_string()
                ))
            }
        }
    }
}
