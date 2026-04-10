use std::fmt;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, SyncSender};
use std::time::Duration;

use anyhow::{anyhow, Error};
use mascot_render_client::MotionTimelineRequest;

type ControlCommandApplyResult = std::result::Result<(), String>;

#[derive(Clone)]
pub struct ControlCommandCompletion {
    tx: SyncSender<ControlCommandApplyResult>,
}

pub struct ControlCommandCompletionWaiter {
    rx: Receiver<ControlCommandApplyResult>,
}

pub(crate) enum ControlCommandWaitError {
    ApplyFailed(String),
    TimedOut(Duration),
    Disconnected,
}

#[derive(Debug)]
pub enum MascotControlCommand {
    Show,
    Hide,
    ChangeSkin {
        png_path: PathBuf,
        completion: Option<ControlCommandCompletion>,
    },
    PlayTimeline {
        request: MotionTimelineRequest,
        completion: Option<ControlCommandCompletion>,
    },
}

impl MascotControlCommand {
    pub fn change_skin(png_path: PathBuf) -> Self {
        Self::ChangeSkin {
            png_path,
            completion: None,
        }
    }

    pub fn play_timeline(request: MotionTimelineRequest) -> Self {
        Self::PlayTimeline {
            request,
            completion: None,
        }
    }

    pub(crate) fn change_skin_with_completion(
        png_path: PathBuf,
        completion: ControlCommandCompletion,
    ) -> Self {
        Self::ChangeSkin {
            png_path,
            completion: Some(completion),
        }
    }

    pub(crate) fn play_timeline_with_completion(
        request: MotionTimelineRequest,
        completion: ControlCommandCompletion,
    ) -> Self {
        Self::PlayTimeline {
            request,
            completion: Some(completion),
        }
    }

    pub fn finish(&self, result: ControlCommandApplyResult) {
        match self {
            Self::ChangeSkin {
                completion: Some(completion),
                ..
            }
            | Self::PlayTimeline {
                completion: Some(completion),
                ..
            } => completion.finish(result),
            Self::Show
            | Self::Hide
            | Self::ChangeSkin {
                completion: None, ..
            }
            | Self::PlayTimeline {
                completion: None, ..
            } => {}
        }
    }
}

impl PartialEq for MascotControlCommand {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Show, Self::Show) | (Self::Hide, Self::Hide) => true,
            (
                Self::ChangeSkin { png_path: left, .. },
                Self::ChangeSkin {
                    png_path: right, ..
                },
            ) => left == right,
            (
                Self::PlayTimeline { request: left, .. },
                Self::PlayTimeline { request: right, .. },
            ) => left == right,
            _ => false,
        }
    }
}

impl Eq for MascotControlCommand {}

impl ControlCommandCompletion {
    pub(crate) fn pair() -> (Self, ControlCommandCompletionWaiter) {
        let (tx, rx) = mpsc::sync_channel(1);
        (Self { tx }, ControlCommandCompletionWaiter { rx })
    }

    fn finish(&self, result: ControlCommandApplyResult) {
        let _ = self.tx.send(result);
    }
}

impl fmt::Debug for ControlCommandCompletion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ControlCommandCompletion(..)")
    }
}

impl ControlCommandCompletionWaiter {
    pub(crate) fn wait(
        self,
        timeout: Duration,
    ) -> std::result::Result<(), ControlCommandWaitError> {
        match self.rx.recv_timeout(timeout) {
            Ok(Ok(())) => Ok(()),
            Ok(Err(message)) => Err(ControlCommandWaitError::ApplyFailed(message)),
            Err(RecvTimeoutError::Timeout) => Err(ControlCommandWaitError::TimedOut(timeout)),
            Err(RecvTimeoutError::Disconnected) => Err(ControlCommandWaitError::Disconnected),
        }
    }
}

impl ControlCommandWaitError {
    pub(crate) fn into_anyhow(self, action: &str) -> Error {
        match self {
            Self::ApplyFailed(message) => anyhow!(
                "mascot {action} command failed while applying in the UI thread: {message}"
            ),
            Self::TimedOut(timeout) => anyhow!(
                "timed out waiting for mascot {action} command to finish applying in the UI thread after {}s",
                timeout.as_secs()
            ),
            Self::Disconnected => anyhow!(
                "mascot {action} command completion channel disconnected before the UI thread reported a result"
            ),
        }
    }
}
