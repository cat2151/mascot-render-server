use std::fmt;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, SyncSender};
use std::time::Duration;

use anyhow::{anyhow, Error};
use mascot_render_protocol::{MotionTimelineRequest, ServerCommandKind, ServerCommandStatus};

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
    Show {
        status: ServerCommandStatus,
    },
    Hide {
        status: ServerCommandStatus,
    },
    ChangeCharacter {
        character_name: String,
        completion: Option<ControlCommandCompletion>,
        status: ServerCommandStatus,
    },
    PlayTimeline {
        request: MotionTimelineRequest,
        completion: Option<ControlCommandCompletion>,
        status: ServerCommandStatus,
    },
}

impl MascotControlCommand {
    pub fn show() -> Self {
        Self::show_with_status(ServerCommandStatus::queued(ServerCommandKind::Show, "show"))
    }

    pub fn hide() -> Self {
        Self::hide_with_status(ServerCommandStatus::queued(ServerCommandKind::Hide, "hide"))
    }

    pub fn change_character(character_name: String) -> Self {
        let summary = change_character_summary(&character_name);
        Self::change_character_with_status(
            character_name,
            None,
            ServerCommandStatus::queued(ServerCommandKind::ChangeCharacter, summary),
        )
    }

    pub fn play_timeline(request: MotionTimelineRequest) -> Self {
        let summary = timeline_summary(&request);
        Self::play_timeline_with_status(
            request,
            None,
            ServerCommandStatus::queued(ServerCommandKind::Timeline, summary),
        )
    }

    pub(crate) fn show_with_status(status: ServerCommandStatus) -> Self {
        Self::Show { status }
    }

    pub(crate) fn hide_with_status(status: ServerCommandStatus) -> Self {
        Self::Hide { status }
    }

    pub(crate) fn change_character_with_completion(
        character_name: String,
        completion: ControlCommandCompletion,
        status: ServerCommandStatus,
    ) -> Self {
        Self::change_character_with_status(character_name, Some(completion), status)
    }

    pub(crate) fn play_timeline_with_completion(
        request: MotionTimelineRequest,
        completion: ControlCommandCompletion,
        status: ServerCommandStatus,
    ) -> Self {
        Self::play_timeline_with_status(request, Some(completion), status)
    }

    fn change_character_with_status(
        character_name: String,
        completion: Option<ControlCommandCompletion>,
        status: ServerCommandStatus,
    ) -> Self {
        Self::ChangeCharacter {
            character_name,
            completion,
            status,
        }
    }

    fn play_timeline_with_status(
        request: MotionTimelineRequest,
        completion: Option<ControlCommandCompletion>,
        status: ServerCommandStatus,
    ) -> Self {
        Self::PlayTimeline {
            request,
            completion,
            status,
        }
    }

    pub fn status(&self) -> &ServerCommandStatus {
        match self {
            Self::Show { status }
            | Self::Hide { status }
            | Self::ChangeCharacter { status, .. }
            | Self::PlayTimeline { status, .. } => status,
        }
    }

    pub fn finish(&self, result: ControlCommandApplyResult) {
        match self {
            Self::ChangeCharacter {
                completion: Some(completion),
                ..
            }
            | Self::PlayTimeline {
                completion: Some(completion),
                ..
            } => completion.finish(result),
            Self::Show { .. }
            | Self::Hide { .. }
            | Self::ChangeCharacter {
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
            (Self::Show { .. }, Self::Show { .. }) | (Self::Hide { .. }, Self::Hide { .. }) => true,
            (
                Self::ChangeCharacter {
                    character_name: left,
                    ..
                },
                Self::ChangeCharacter {
                    character_name: right,
                    ..
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

pub(crate) fn change_character_summary(character_name: &str) -> String {
    format!("character={character_name}")
}

pub(crate) fn timeline_summary(request: &MotionTimelineRequest) -> String {
    let Some(step) = request.steps.first() else {
        return "timeline steps=0".to_string();
    };
    format!(
        "{:?} duration_ms={} fps={}",
        step.kind, step.duration_ms, step.fps
    )
}
