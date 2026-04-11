use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

use anyhow::Result as AnyhowResult;
use mascot_render_client::{
    change_skin_mascot_render_server, hide_mascot_render_server,
    play_timeline_mascot_render_server, preview_mouth_flap_timeline_request,
    show_mascot_render_server,
};
use mascot_render_control::ensure_mascot_render_server_running;
use mascot_render_protocol::{MotionTimelineKind, MotionTimelineRequest, MotionTimelineStep};

const TEST_SHAKE_DURATION_MS: u64 = 900;
const TEST_TIMELINE_FPS: u16 = 20;

#[derive(Debug)]
pub(crate) enum TestPostAction {
    Show,
    Hide,
    ChangeSkin(PathBuf),
    ShakeTimeline,
    MouthFlapTimeline,
}

#[derive(Debug)]
pub(crate) struct TestPostSync {
    config_path: PathBuf,
    result_rx: Option<Receiver<TestPostResult>>,
}

type TestPostResult = std::result::Result<String, (String, String)>;

impl TestPostSync {
    pub(crate) fn new(config_path: PathBuf) -> Self {
        Self {
            config_path,
            result_rx: None,
        }
    }

    pub(crate) fn start_if_idle(
        &mut self,
        action: TestPostAction,
    ) -> std::result::Result<(), String> {
        if self.result_rx.is_some() {
            return Err("another POST is still running".to_string());
        }

        let label = action.label();
        let config_path = self.config_path.clone();
        let (result_tx, result_rx) = mpsc::channel();
        self.result_rx = Some(result_rx);

        thread::spawn(move || {
            let result = run_test_post_action(&config_path, action)
                .map(|()| label.clone())
                .map_err(|error| (label, format!("{error:#}")));
            let _ = result_tx.send(result);
        });

        Ok(())
    }

    pub(crate) fn drain_completion(&mut self) -> Option<TestPostResult> {
        let result_rx = self.result_rx.as_ref()?;
        match result_rx.try_recv() {
            Ok(result) => {
                self.result_rx = None;
                Some(result)
            }
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                self.result_rx = None;
                Some(Err((
                    "POST".to_string(),
                    "mascot-render-server POST worker disconnected".to_string(),
                )))
            }
        }
    }
}

impl TestPostAction {
    pub(crate) fn label(&self) -> String {
        match self {
            Self::Show => "show".to_string(),
            Self::Hide => "hide".to_string(),
            Self::ChangeSkin(_) => Self::change_skin_label(),
            Self::ShakeTimeline => "timeline shake".to_string(),
            Self::MouthFlapTimeline => "timeline mouth-flap".to_string(),
        }
    }

    pub(crate) fn change_skin_label() -> String {
        "change-skin current_png_path".to_string()
    }
}

fn run_test_post_action(config_path: &Path, action: TestPostAction) -> AnyhowResult<()> {
    ensure_mascot_render_server_running(config_path)?;

    match action {
        TestPostAction::Show => show_mascot_render_server(),
        TestPostAction::Hide => hide_mascot_render_server(),
        TestPostAction::ChangeSkin(png_path) => change_skin_mascot_render_server(&png_path),
        TestPostAction::ShakeTimeline => {
            play_timeline_mascot_render_server(&shake_timeline_request())
        }
        TestPostAction::MouthFlapTimeline => {
            play_timeline_mascot_render_server(&preview_mouth_flap_timeline_request())
        }
    }
}

pub(crate) fn shake_timeline_request() -> MotionTimelineRequest {
    MotionTimelineRequest {
        steps: vec![MotionTimelineStep {
            kind: MotionTimelineKind::Shake,
            duration_ms: TEST_SHAKE_DURATION_MS,
            fps: TEST_TIMELINE_FPS,
        }],
    }
}
