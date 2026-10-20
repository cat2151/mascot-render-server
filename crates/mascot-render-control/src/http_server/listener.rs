use super::*;
use std::net::TcpListener;

pub(super) fn bind_control_listener(address: SocketAddr) -> Result<TcpListener> {
    TcpListener::bind(address).map_err(|error| match error.kind() {
        std::io::ErrorKind::AddrInUse => anyhow!(
            "failed to bind {address}: mascot-render-server may already be running on this port; reuse the existing server or stop it first"
        ),
        _ => anyhow!("failed to bind {address}: {error}"),
    })
}

pub(super) fn accept_loop(
    listener: TcpListener,
    command_tx: Sender<MascotControlCommand>,
    status_store: ServerStatusStore,
    psd_file_names: Arc<PsdFileNamesLoader>,
    notify: Option<Arc<dyn Fn() + Send + Sync>>,
) {
    loop {
        match listener.accept() {
            Ok((mut stream, peer)) => {
                if let Err(error) = stream.set_read_timeout(Some(IO_TIMEOUT)) {
                    log_control_error(format!(
                        "event=control_connection stage=set_read_timeout peer={peer} error={error}"
                    ));
                    continue;
                }
                if let Err(error) = stream.set_write_timeout(Some(IO_TIMEOUT)) {
                    log_control_error(format!(
                        "event=control_connection stage=set_write_timeout peer={peer} error={error}"
                    ));
                    continue;
                }
                if let Err(error) = handle_connection(
                    &mut stream,
                    peer,
                    &command_tx,
                    &status_store,
                    psd_file_names.as_ref(),
                    notify.as_ref(),
                ) {
                    log_control_error(format!(
                        "event=control_connection stage=handle peer={peer} error={error:#}"
                    ));
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(ACCEPT_POLL_INTERVAL);
            }
            Err(error) => {
                log_control_error(format!("event=control_accept stage=accept error={error}"));
                thread::sleep(ACCEPT_POLL_INTERVAL);
            }
        }
    }
}
