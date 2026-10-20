use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Result};

use crate::{now_unix_ms, ServerStatusSnapshot};

#[derive(Clone)]
pub struct ServerStatusStore {
    inner: Arc<Mutex<ServerStatusSnapshot>>,
    playback: Arc<Mutex<crate::playback::PlaybackSlot>>,
}

impl ServerStatusStore {
    pub fn new(snapshot: ServerStatusSnapshot) -> Self {
        Self {
            inner: Arc::new(Mutex::new(snapshot)),
            playback: Default::default(),
        }
    }

    pub fn set_playback(&self, request: crate::PlaybackRequest) -> Result<()> {
        self.playback
            .lock()
            .map_err(|e| anyhow!("playback store poisoned: {e}"))?
            .update(request)
    }
    pub fn playback(&self) -> Result<Option<crate::PlaybackSnapshot>> {
        Ok(self
            .playback
            .lock()
            .map_err(|e| anyhow!("playback store poisoned: {e}"))?
            .0
            .clone())
    }

    pub fn snapshot(&self) -> Result<ServerStatusSnapshot> {
        self.update(|snapshot| {
            snapshot.captured_at_unix_ms = now_unix_ms();
        })?;
        self.inner
            .lock()
            .map(|snapshot| snapshot.clone())
            .map_err(|error| anyhow!("server status store is poisoned: {error}"))
    }

    pub fn update(&self, update: impl FnOnce(&mut ServerStatusSnapshot)) -> Result<()> {
        let mut snapshot = self
            .inner
            .lock()
            .map_err(|error| anyhow!("server status store is poisoned: {error}"))?;
        update(&mut snapshot);
        Ok(())
    }
}
