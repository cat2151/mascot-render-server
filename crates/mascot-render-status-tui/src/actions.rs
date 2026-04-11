use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use anyhow::{bail, Context, Result as AnyhowResult};
use mascot_render_client::{
    change_character_mascot_render_server, hide_mascot_render_server,
    play_timeline_mascot_render_server, preview_mouth_flap_timeline_request,
    show_mascot_render_server,
};
use mascot_render_control::ensure_mascot_render_server_running;
use mascot_render_core::{Core, CoreConfig, PsdEntry, ZipEntry};
use mascot_render_protocol::{MotionTimelineKind, MotionTimelineRequest, MotionTimelineStep};

const TEST_SHAKE_DURATION_MS: u64 = 900;
const TEST_TIMELINE_FPS: u16 = 20;

#[derive(Debug)]
pub(crate) enum TestPostAction {
    Show,
    Hide,
    ChangeCharacter(String),
    RandomCharacter { current: Option<CachedPsdSource> },
    ShakeTimeline,
    MouthFlapTimeline,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CachedPsdSource {
    pub(crate) zip_path: PathBuf,
    pub(crate) psd_path_in_zip: PathBuf,
    pub(crate) png_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RandomCharacterSelection {
    pub(crate) character_name: String,
    pub(crate) source: CachedPsdSource,
    pub(crate) candidate_count: usize,
    pub(crate) selectable_count: usize,
}

#[derive(Debug)]
pub(crate) struct TestPostSync {
    config_path: PathBuf,
    result_rx: Option<Receiver<TestPostResult>>,
}

type TestPostResult = std::result::Result<(String, u64), (String, String)>;

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
            Self::ChangeCharacter(_) => Self::change_character_label(),
            Self::RandomCharacter { .. } => Self::random_character_label(),
            Self::ShakeTimeline => "timeline shake".to_string(),
            Self::MouthFlapTimeline => "timeline mouth-flap".to_string(),
        }
    }

    pub(crate) fn change_character_label() -> String {
        "change-character configured_character_name".to_string()
    }

    pub(crate) fn random_character_label() -> String {
        "change-character random cached PSD".to_string()
    }
}

fn run_test_post_action(
    config_path: &std::path::Path,
    action: TestPostAction,
) -> AnyhowResult<(String, u64)> {
    ensure_mascot_render_server_running(config_path)?;

    let started_at = Instant::now();
    let label = match action {
        TestPostAction::Show => {
            show_mascot_render_server()?;
            TestPostAction::Show.label()
        }
        TestPostAction::Hide => {
            hide_mascot_render_server()?;
            TestPostAction::Hide.label()
        }
        TestPostAction::ChangeCharacter(character_name) => {
            change_character_mascot_render_server(&character_name)?;
            TestPostAction::change_character_label()
        }
        TestPostAction::RandomCharacter { current } => {
            let selection = select_random_character_from_cache(current.as_ref())?;
            change_character_mascot_render_server(&selection.character_name)?;
            random_character_selection_label(&selection)
        }
        TestPostAction::ShakeTimeline => {
            play_timeline_mascot_render_server(&shake_timeline_request())?;
            TestPostAction::ShakeTimeline.label()
        }
        TestPostAction::MouthFlapTimeline => {
            play_timeline_mascot_render_server(&preview_mouth_flap_timeline_request())?;
            TestPostAction::MouthFlapTimeline.label()
        }
    };

    Ok((label, elapsed_ms_since(started_at)))
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

fn elapsed_ms_since(started_at: Instant) -> u64 {
    u64::try_from(started_at.elapsed().as_millis()).unwrap_or(u64::MAX)
}

fn select_random_character_from_cache(
    current: Option<&CachedPsdSource>,
) -> AnyhowResult<RandomCharacterSelection> {
    let core = Core::new(CoreConfig::default());
    let cache_dir = core.cache_dir().to_path_buf();
    let entries = core.load_cached_zip_entries_snapshot().with_context(|| {
        format!(
            "failed to load cached PSD list from {}",
            cache_dir.display()
        )
    })?;
    select_random_character_candidate(cached_psd_candidates(&entries), current, random_seed())
        .with_context(|| format!("cache_dir={}", cache_dir.display()))
}

pub(crate) fn cached_psd_candidates(entries: &[ZipEntry]) -> Vec<RandomCharacterSelection> {
    entries
        .iter()
        .flat_map(|zip_entry| {
            zip_entry
                .psds
                .iter()
                .filter_map(move |psd| cached_psd_candidate(zip_entry, psd))
        })
        .collect()
}

pub(crate) fn select_random_character_candidate(
    candidates: Vec<RandomCharacterSelection>,
    current: Option<&CachedPsdSource>,
    seed: u64,
) -> AnyhowResult<RandomCharacterSelection> {
    let candidate_count = candidates.len();
    if candidate_count == 0 {
        bail!("no cached PSD entries could generate a change-character name");
    }

    let selectable = selectable_character_candidates(candidates, current);
    let selectable_count = selectable.len();
    let selected_index = candidate_index_from_seed(selectable_count, seed);
    let mut selected = selectable[selected_index].clone();
    selected.candidate_count = candidate_count;
    selected.selectable_count = selectable_count;
    Ok(selected)
}

fn cached_psd_candidate(zip_entry: &ZipEntry, psd: &PsdEntry) -> Option<RandomCharacterSelection> {
    let source = CachedPsdSource {
        png_path: psd.rendered_png_path.clone()?,
        zip_path: zip_entry.zip_path.clone(),
        psd_path_in_zip: psd_path_in_zip(zip_entry, psd),
    };
    Some(RandomCharacterSelection {
        character_name: generated_character_name(&source.zip_path, &source.psd_path_in_zip)?,
        source,
        candidate_count: 0,
        selectable_count: 0,
    })
}

fn selectable_character_candidates(
    candidates: Vec<RandomCharacterSelection>,
    current: Option<&CachedPsdSource>,
) -> Vec<RandomCharacterSelection> {
    if candidates.len() <= 1 {
        return candidates;
    }

    let Some(current) = current else {
        return candidates;
    };

    let filtered = candidates
        .iter()
        .filter(|candidate| !same_psd_source(&candidate.source, current))
        .cloned()
        .collect::<Vec<_>>();
    if filtered.is_empty() {
        candidates
    } else {
        filtered
    }
}

fn same_psd_source(left: &CachedPsdSource, right: &CachedPsdSource) -> bool {
    left.png_path == right.png_path
        && left.zip_path == right.zip_path
        && left.psd_path_in_zip == right.psd_path_in_zip
}

fn generated_character_name(zip_path: &Path, psd_path_in_zip: &Path) -> Option<String> {
    let zip_text = searchable_path_text(zip_path);
    path_search_tokens(psd_path_in_zip)
        .into_iter()
        .filter(|token| zip_text.contains(token))
        .max_by_key(|token| token.chars().count())
}

fn searchable_path_text(path: &Path) -> String {
    let mut text = path.to_string_lossy().to_string();
    if let Some(file_name) = path.file_name() {
        text.push(' ');
        text.push_str(&file_name.to_string_lossy());
    }
    text
}

fn path_search_tokens(path: &Path) -> Vec<String> {
    path.components()
        .filter_map(|component| {
            let token = component.as_os_str().to_string_lossy();
            let token = token.trim();
            (!token.is_empty()).then(|| trim_psd_extension(token).to_string())
        })
        .filter(|token| !token.is_empty())
        .collect()
}

fn trim_psd_extension(token: &str) -> &str {
    match token.rsplit_once('.') {
        Some((stem, ext)) if ext.eq_ignore_ascii_case("psd") => stem,
        _ => token,
    }
}

fn psd_path_in_zip(zip_entry: &ZipEntry, psd: &PsdEntry) -> PathBuf {
    psd.path
        .strip_prefix(&zip_entry.extracted_dir)
        .map(Path::to_path_buf)
        .unwrap_or_else(|_| psd.path.clone())
}

fn random_seed() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64
}

fn candidate_index_from_seed(candidate_count: usize, seed: u64) -> usize {
    let mut value = seed.max(1);
    value ^= value >> 12;
    value ^= value << 25;
    value ^= value >> 27;
    (value.wrapping_mul(2_685_821_657_736_338_717) as usize) % candidate_count
}

fn random_character_selection_label(selection: &RandomCharacterSelection) -> String {
    format!(
        "change-character random cached PSD: generated_character_name={} eligible_psd_count={} selectable_psd_count={} random_zip={} random_psd={} random_png={}",
        selection.character_name,
        selection.candidate_count,
        selection.selectable_count,
        selection.source.zip_path.display(),
        selection.source.psd_path_in_zip.display(),
        selection.source.png_path.display()
    )
}
