use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};

use mascot_render_client::{MotionTimelineKind, MotionTimelineRequest, MotionTimelineStep};
use mascot_render_core::mascot_config_path;
use mascot_render_server::play_mascot_render_server_timeline;

const SERVER_MOTION_ACTIVITY_MESSAGE: &str = "Sending mascot shake timeline...";
const SERVER_MOTION_ACTIVITY_MIN_VISIBLE: Duration = Duration::from_millis(250);
const SHAKE_DURATION_MS: u64 = 5_000;
const SHAKE_FPS: u16 = 20;

#[derive(Debug)]
struct ServerMotionSyncEvent {
    generation: u64,
    result: anyhow::Result<()>,
}

#[derive(Debug)]
pub(crate) struct ServerMotionSync {
    generation: u64,
    active_generation: Option<u64>,
    activity_visible_until: Option<Instant>,
    result_tx: Sender<ServerMotionSyncEvent>,
    result_rx: Receiver<ServerMotionSyncEvent>,
}

impl ServerMotionSync {
    pub(crate) fn new() -> Self {
        let (result_tx, result_rx) = mpsc::channel();
        Self {
            generation: 0,
            active_generation: None,
            activity_visible_until: None,
            result_tx,
            result_rx,
        }
    }

    pub(crate) fn request_shake(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.active_generation = Some(self.generation);
        self.activity_visible_until = Some(Instant::now() + SERVER_MOTION_ACTIVITY_MIN_VISIBLE);
        let generation = self.generation;
        let result_tx = self.result_tx.clone();
        let request = shake_timeline_request();
        thread::spawn(move || {
            let result = play_mascot_render_server_timeline(&mascot_config_path(), &request);
            let _ = result_tx.send(ServerMotionSyncEvent { generation, result });
        });
    }

    pub(crate) fn drain_completions(&mut self) -> Option<anyhow::Error> {
        loop {
            match self.result_rx.try_recv() {
                Ok(event) if Some(event.generation) != self.active_generation => continue,
                Ok(event) => {
                    self.active_generation = None;
                    if let Err(error) = event.result {
                        return Some(error);
                    }
                }
                Err(TryRecvError::Empty) | Err(TryRecvError::Disconnected) => return None,
            }
        }
    }

    pub(crate) fn activity_message(&self) -> Option<&'static str> {
        let still_visible = self
            .activity_visible_until
            .is_some_and(|deadline| Instant::now() < deadline);
        (self.active_generation.is_some() || still_visible)
            .then_some(SERVER_MOTION_ACTIVITY_MESSAGE)
    }
}

pub(crate) fn shake_requested_status_message() -> &'static str {
    "Mascot shake requested: 5s @ 20fps."
}

fn shake_timeline_request() -> MotionTimelineRequest {
    MotionTimelineRequest {
        steps: vec![MotionTimelineStep {
            kind: MotionTimelineKind::Shake,
            duration_ms: SHAKE_DURATION_MS,
            fps: SHAKE_FPS,
        }],
    }
}
