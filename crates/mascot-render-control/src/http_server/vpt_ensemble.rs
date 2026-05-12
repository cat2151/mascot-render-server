use std::net::SocketAddr;
use std::sync::mpsc::Sender;
use std::sync::Arc;

use anyhow::{Context, Result};
use mascot_render_protocol::{
    validate_vpt_ensemble_request, ServerCommandKind, ServerCommandStatus, ServerStatusStore,
    VptEnsembleRequest,
};

use super::http_protocol::HttpResponse;
use super::{enqueue_apply_command, log_request_payload};
use crate::command::{vpt_ensemble_summary, MascotControlCommand};
use crate::logging::log_control_info;

pub(super) fn route_set_vpt_ensemble(
    peer: SocketAddr,
    body: &[u8],
    command_tx: &Sender<MascotControlCommand>,
    status_store: &ServerStatusStore,
    notify: Option<&Arc<dyn Fn() + Send + Sync>>,
) -> Result<HttpResponse> {
    let request = parse_vpt_ensemble_request(body, "vpt ensemble")?;
    let status = ServerCommandStatus::queued(
        ServerCommandKind::SetVptEnsemble,
        vpt_ensemble_summary(&request),
    );
    log_request_payload(peer, "set_vpt_ensemble", &request);
    let response = enqueue_apply_command(
        peer,
        "set_vpt_ensemble",
        command_tx,
        status_store,
        notify,
        |completion| {
            MascotControlCommand::set_vpt_ensemble_with_completion(
                request.clone(),
                completion,
                status,
            )
        },
    )?;
    log_applied(peer, "set_vpt_ensemble", request.character_names.len());
    Ok(response)
}

pub(super) fn route_set_vpt_ensemble_members(
    peer: SocketAddr,
    body: &[u8],
    command_tx: &Sender<MascotControlCommand>,
    status_store: &ServerStatusStore,
    notify: Option<&Arc<dyn Fn() + Send + Sync>>,
) -> Result<HttpResponse> {
    let request = parse_vpt_ensemble_request(body, "vpt ensemble members")?;
    let status = ServerCommandStatus::queued(
        ServerCommandKind::SetVptEnsembleMembers,
        vpt_ensemble_summary(&request),
    );
    log_request_payload(peer, "set_vpt_ensemble_members", &request);
    let response = enqueue_apply_command(
        peer,
        "set_vpt_ensemble_members",
        command_tx,
        status_store,
        notify,
        |completion| {
            MascotControlCommand::set_vpt_ensemble_members_with_completion(
                request.clone(),
                completion,
                status,
            )
        },
    )?;
    log_applied(
        peer,
        "set_vpt_ensemble_members",
        request.character_names.len(),
    );
    Ok(response)
}

fn parse_vpt_ensemble_request(body: &[u8], label: &str) -> Result<VptEnsembleRequest> {
    let request: VptEnsembleRequest = serde_json::from_slice(body)
        .with_context(|| format!("failed to parse mascot {label} request JSON"))?;
    validate_vpt_ensemble_request(&request)?;
    Ok(request)
}

fn log_applied(peer: SocketAddr, action: &str, character_count: usize) {
    log_control_info(format!(
        "event=control_request stage=applied peer={peer} action={action} character_count={character_count}"
    ));
}
