use std::fmt;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, SyncSender};
use std::time::Duration;

use anyhow::{anyhow, Error};
use mascot_render_protocol::{
    MotionTimelineRequest, PreviewTargetRequest, ServerCommandKind, ServerCommandStatus,
    ServerEnsembleMode, VptEnsembleRequest,
};

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
    PreviewTarget {
        request: PreviewTargetRequest,
        completion: Option<ControlCommandCompletion>,
        status: ServerCommandStatus,
    },
    SetEnsembleMode {
        mode: ServerEnsembleMode,
        completion: Option<ControlCommandCompletion>,
        status: ServerCommandStatus,
    },
    SetVptEnsemble {
        request: VptEnsembleRequest,
        completion: Option<ControlCommandCompletion>,
        status: ServerCommandStatus,
    },
    SetVptEnsembleMembers {
        request: VptEnsembleRequest,
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

    pub fn preview_target(request: PreviewTargetRequest) -> Self {
        let summary = preview_target_summary(&request);
        Self::preview_target_with_status(
            request,
            None,
            ServerCommandStatus::queued(ServerCommandKind::PreviewTarget, summary),
        )
    }

    pub fn set_ensemble_mode(mode: ServerEnsembleMode) -> Self {
        Self::set_ensemble_mode_with_status(
            mode,
            None,
            ServerCommandStatus::queued(
                ServerCommandKind::SetEnsembleMode,
                set_ensemble_mode_summary(mode),
            ),
        )
    }

    pub fn set_vpt_ensemble(request: VptEnsembleRequest) -> Self {
        let summary = vpt_ensemble_summary(&request);
        Self::set_vpt_ensemble_with_status(
            request,
            None,
            ServerCommandStatus::queued(ServerCommandKind::SetVptEnsemble, summary),
        )
    }

    pub fn set_vpt_ensemble_members(request: VptEnsembleRequest) -> Self {
        let summary = vpt_ensemble_summary(&request);
        Self::set_vpt_ensemble_members_with_status(
            request,
            None,
            ServerCommandStatus::queued(ServerCommandKind::SetVptEnsembleMembers, summary),
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

    pub(crate) fn preview_target_with_completion(
        request: PreviewTargetRequest,
        completion: ControlCommandCompletion,
        status: ServerCommandStatus,
    ) -> Self {
        Self::preview_target_with_status(request, Some(completion), status)
    }

    pub(crate) fn set_ensemble_mode_with_completion(
        mode: ServerEnsembleMode,
        completion: ControlCommandCompletion,
        status: ServerCommandStatus,
    ) -> Self {
        Self::set_ensemble_mode_with_status(mode, Some(completion), status)
    }

    pub(crate) fn set_vpt_ensemble_with_completion(
        request: VptEnsembleRequest,
        completion: ControlCommandCompletion,
        status: ServerCommandStatus,
    ) -> Self {
        Self::set_vpt_ensemble_with_status(request, Some(completion), status)
    }

    pub(crate) fn set_vpt_ensemble_members_with_completion(
        request: VptEnsembleRequest,
        completion: ControlCommandCompletion,
        status: ServerCommandStatus,
    ) -> Self {
        Self::set_vpt_ensemble_members_with_status(request, Some(completion), status)
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

    fn preview_target_with_status(
        request: PreviewTargetRequest,
        completion: Option<ControlCommandCompletion>,
        status: ServerCommandStatus,
    ) -> Self {
        Self::PreviewTarget {
            request,
            completion,
            status,
        }
    }

    fn set_ensemble_mode_with_status(
        mode: ServerEnsembleMode,
        completion: Option<ControlCommandCompletion>,
        status: ServerCommandStatus,
    ) -> Self {
        Self::SetEnsembleMode {
            mode,
            completion,
            status,
        }
    }

    fn set_vpt_ensemble_with_status(
        request: VptEnsembleRequest,
        completion: Option<ControlCommandCompletion>,
        status: ServerCommandStatus,
    ) -> Self {
        Self::SetVptEnsemble {
            request,
            completion,
            status,
        }
    }

    fn set_vpt_ensemble_members_with_status(
        request: VptEnsembleRequest,
        completion: Option<ControlCommandCompletion>,
        status: ServerCommandStatus,
    ) -> Self {
        Self::SetVptEnsembleMembers {
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
            | Self::PreviewTarget { status, .. }
            | Self::SetEnsembleMode { status, .. }
            | Self::SetVptEnsemble { status, .. }
            | Self::SetVptEnsembleMembers { status, .. }
            | Self::PlayTimeline { status, .. } => status,
        }
    }

    pub fn finish(&self, result: ControlCommandApplyResult) {
        match self {
            Self::ChangeCharacter {
                completion: Some(completion),
                ..
            }
            | Self::PreviewTarget {
                completion: Some(completion),
                ..
            }
            | Self::SetEnsembleMode {
                completion: Some(completion),
                ..
            }
            | Self::SetVptEnsemble {
                completion: Some(completion),
                ..
            }
            | Self::SetVptEnsembleMembers {
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
            | Self::PreviewTarget {
                completion: None, ..
            }
            | Self::SetEnsembleMode {
                completion: None, ..
            }
            | Self::SetVptEnsemble {
                completion: None, ..
            }
            | Self::SetVptEnsembleMembers {
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
                Self::PreviewTarget { request: left, .. },
                Self::PreviewTarget { request: right, .. },
            ) => left == right,
            (
                Self::PlayTimeline { request: left, .. },
                Self::PlayTimeline { request: right, .. },
            ) => left == right,
            (
                Self::SetEnsembleMode { mode: left, .. },
                Self::SetEnsembleMode { mode: right, .. },
            ) => left == right,
            (
                Self::SetVptEnsemble { request: left, .. },
                Self::SetVptEnsemble { request: right, .. },
            ) => left == right,
            (
                Self::SetVptEnsembleMembers { request: left, .. },
                Self::SetVptEnsembleMembers { request: right, .. },
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
    let target = request.target_character_name.as_deref().unwrap_or("-");
    format!(
        "{:?} duration_ms={} fps={} target_character_name={target}",
        step.kind, step.duration_ms, step.fps
    )
}

pub(crate) fn preview_target_summary(request: &PreviewTargetRequest) -> String {
    format!(
        "png={} zip={} psd={} display_diff={} scale={}",
        request.png_path.display(),
        request.zip_path.display(),
        request.psd_path_in_zip.display(),
        optional_path_text(request.display_diff_path.as_ref()),
        optional_scale_text(request.scale)
    )
}

pub(crate) fn set_ensemble_mode_summary(mode: ServerEnsembleMode) -> String {
    format!("ensemble_mode={mode:?}")
}

pub(crate) fn vpt_ensemble_summary(request: &VptEnsembleRequest) -> String {
    format!(
        "vpt_ensemble character_count={}",
        request.character_names.len()
    )
}

fn optional_path_text(path: Option<&std::path::PathBuf>) -> String {
    path.map(|path| path.display().to_string())
        .unwrap_or_else(|| "-".to_string())
}

fn optional_scale_text(scale: Option<f32>) -> String {
    scale
        .map(|value| format!("{value:.3}"))
        .unwrap_or_else(|| "-".to_string())
}
