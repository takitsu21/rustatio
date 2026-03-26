use crate::protocol::PeerHandshake;
use crate::{log_debug, log_info, log_warn, FakerState, RatioFakerHandle};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, watch};

const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);
const KEEP_OPEN_DELAY: Duration = Duration::from_secs(2);
const BIND_RETRY_DELAY: Duration = Duration::from_millis(500);

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PeerListenerStatus {
    pub enabled: bool,
    pub desired_port: Option<u16>,
    pub bound_port: Option<u16>,
    pub active_torrents: usize,
    pub last_error: Option<String>,
}

#[async_trait]
pub trait PeerLookup: Send + Sync {
    async fn snapshot(&self) -> PeerCatalog;
}

pub type PeerCatalog = HashMap<[u8; 20], Result<Arc<RatioFakerHandle>, String>>;

pub struct PeerListenerService {
    desired_port_tx: watch::Sender<Option<u16>>,
    status_tx: watch::Sender<PeerListenerStatus>,
    shutdown_tx: Option<mpsc::Sender<()>>,
    task_handle: Option<tokio::task::JoinHandle<()>>,
}

impl PeerListenerService {
    pub fn new() -> Self {
        let (desired_port_tx, _) = watch::channel(None);
        let (status_tx, _) = watch::channel(PeerListenerStatus::default());
        Self { desired_port_tx, status_tx, shutdown_tx: None, task_handle: None }
    }

    pub fn status(&self) -> PeerListenerStatus {
        self.status_tx.borrow().clone()
    }

    pub fn subscribe(&self) -> watch::Receiver<PeerListenerStatus> {
        self.status_tx.subscribe()
    }

    pub fn start(&mut self, lookup: Arc<dyn PeerLookup>) {
        if self.task_handle.is_some() {
            return;
        }

        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
        let desired_port_rx = self.desired_port_tx.subscribe();
        let status_tx = self.status_tx.clone();
        let task = tokio::spawn(async move {
            run_listener_loop(desired_port_rx, status_tx, lookup, shutdown_rx).await;
        });

        self.shutdown_tx = Some(shutdown_tx);
        self.task_handle = Some(task);
    }

    pub fn set_desired_port(&self, port: Option<u16>) {
        let _ = self.desired_port_tx.send(port);
    }

    pub async fn shutdown(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(()).await;
        }
        if let Some(handle) = self.task_handle.take() {
            let _ = tokio::time::timeout(Duration::from_secs(5), handle).await;
        }
    }
}

impl Default for PeerListenerService {
    fn default() -> Self {
        Self::new()
    }
}

async fn run_listener_loop(
    mut desired_port_rx: watch::Receiver<Option<u16>>,
    status_tx: watch::Sender<PeerListenerStatus>,
    lookup: Arc<dyn PeerLookup>,
    mut shutdown_rx: mpsc::Receiver<()>,
) {
    let mut bound_port: Option<u16> = None;
    let mut listener: Option<TcpListener> = None;

    loop {
        let desired = *desired_port_rx.borrow();
        let catalog = lookup.snapshot().await;
        let active_torrents = catalog.len();

        if desired == bound_port {
            let current = status_tx.borrow().clone();
            if current.active_torrents != active_torrents || current.desired_port != desired {
                let _ = status_tx.send(PeerListenerStatus {
                    enabled: desired.is_some(),
                    desired_port: desired,
                    bound_port,
                    active_torrents,
                    last_error: current.last_error,
                });
            }
        } else {
            listener = None;
            bound_port = None;

            if let Some(port) = desired {
                match TcpListener::bind(("0.0.0.0", port)).await {
                    Ok(sock) => {
                        bound_port = Some(port);
                        listener = Some(sock);
                        log_info!("Peer listener bound on port {}", port);
                        let _ = status_tx.send(PeerListenerStatus {
                            enabled: true,
                            desired_port: desired,
                            bound_port,
                            active_torrents,
                            last_error: None,
                        });
                    }
                    Err(err) => {
                        log_warn!("Failed to bind peer listener on port {}: {}", port, err);
                        let _ = status_tx.send(PeerListenerStatus {
                            enabled: true,
                            desired_port: desired,
                            bound_port: None,
                            active_torrents,
                            last_error: Some(err.to_string()),
                        });
                    }
                }
            } else {
                let _ = status_tx.send(PeerListenerStatus {
                    enabled: false,
                    desired_port: None,
                    bound_port: None,
                    active_torrents,
                    last_error: None,
                });
            }
        }

        match listener.as_mut() {
            Some(sock) => {
                tokio::select! {
                    _ = shutdown_rx.recv() => break,
                    changed = desired_port_rx.changed() => {
                        if changed.is_err() {
                            break;
                        }
                    }
                    accepted = sock.accept() => {
                        match accepted {
                            Ok((stream, addr)) => {
                                let task_catalog = catalog.clone();
                                tokio::spawn(async move {
                                    if let Err(err) = handle_peer(stream, task_catalog).await {
                                        log_debug!("Peer connection from {} closed: {}", addr, err);
                                    }
                                });
                            }
                            Err(err) => {
                                log_warn!("Peer listener accept failed: {}", err);
                                let current = status_tx.borrow().clone();
                                let _ = status_tx.send(PeerListenerStatus {
                                    last_error: Some(err.to_string()),
                                    ..current
                                });
                                tokio::time::sleep(Duration::from_millis(250)).await;
                            }
                        }
                    }
                }
            }
            None => {
                tokio::select! {
                    _ = shutdown_rx.recv() => break,
                    changed = desired_port_rx.changed() => {
                        if changed.is_err() {
                            break;
                        }
                    }
                    () = tokio::time::sleep(BIND_RETRY_DELAY), if desired.is_some() => {}
                }
            }
        }
    }
}

async fn handle_peer(mut stream: TcpStream, catalog: PeerCatalog) -> Result<(), String> {
    let mut buf = [0u8; 68];
    tokio::time::timeout(HANDSHAKE_TIMEOUT, stream.read_exact(&mut buf))
        .await
        .map_err(|_| "handshake read timed out".to_string())?
        .map_err(|e| e.to_string())?;

    let hs = PeerHandshake::from_bytes(&buf).map_err(|e| e.to_string())?;
    let Some(faker) = catalog.get(&hs.info_hash) else {
        return Err("unknown info_hash".to_string());
    };

    let faker = faker.as_ref().map_err(Clone::clone)?;

    if !faker.is_peer_connectable().await {
        return Err("instance not connectable".to_string());
    }

    let peer_id = faker.peer_id_bytes().await.map_err(|e| e.to_string())?;
    let reply = PeerHandshake::new([0u8; 8], hs.info_hash, peer_id).to_bytes();
    stream.write_all(&reply).await.map_err(|e| e.to_string())?;
    stream.flush().await.map_err(|e| e.to_string())?;
    tokio::time::sleep(KEEP_OPEN_DELAY).await;
    let _ = stream.shutdown().await;
    Ok(())
}

pub const fn handle_is_connectable(state: FakerState) -> bool {
    matches!(state, FakerState::Running | FakerState::Paused)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::peer_id_to_array;
    use crate::{FakerConfig, RatioFaker, TorrentFile, TorrentInfo};

    struct StaticLookup {
        catalog: PeerCatalog,
    }

    #[async_trait]
    impl PeerLookup for StaticLookup {
        async fn snapshot(&self) -> PeerCatalog {
            self.catalog.clone()
        }
    }

    fn torrent() -> Arc<TorrentInfo> {
        Arc::new(TorrentInfo {
            info_hash: [7u8; 20],
            announce: "https://tracker.test/announce".to_string(),
            announce_list: None,
            name: "sample".to_string(),
            total_size: 1024,
            piece_length: 1024,
            num_pieces: 1,
            creation_date: None,
            comment: None,
            created_by: None,
            is_single_file: true,
            file_count: 1,
            files: vec![TorrentFile { path: vec!["sample.bin".to_string()], length: 1024 }],
        })
    }

    fn faker_handle() -> Arc<RatioFakerHandle> {
        let faker = RatioFaker::new(torrent(), FakerConfig::default(), None);
        assert!(faker.is_ok());
        Arc::new(RatioFakerHandle::new(faker.unwrap_or_else(|_| unreachable!())))
    }

    fn free_port() -> u16 {
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0));
        assert!(listener.is_ok());
        let listener = listener.unwrap_or_else(|_| unreachable!());
        let addr = listener.local_addr();
        assert!(addr.is_ok());
        addr.unwrap_or_else(|_| unreachable!()).port()
    }

    #[tokio::test]
    async fn connectable_state_allows_running_and_paused() {
        assert!(handle_is_connectable(FakerState::Running));
        assert!(handle_is_connectable(FakerState::Paused));
        assert!(!handle_is_connectable(FakerState::Stopped));
    }

    #[tokio::test]
    async fn listener_binds_and_reports_status() {
        let mut svc = PeerListenerService::new();
        let lookup = Arc::new(StaticLookup { catalog: HashMap::new() });
        svc.start(lookup);
        let mut status_rx = svc.subscribe();
        let port = free_port();
        svc.set_desired_port(Some(port));
        let status = tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                if status_rx.changed().await.is_err() {
                    break svc.status();
                }
                let status = status_rx.borrow().clone();
                if status.bound_port == Some(port) {
                    break status;
                }
            }
        })
        .await
        .unwrap_or_else(|_| unreachable!());
        assert!(status.enabled);
        assert_eq!(status.desired_port, Some(port));
        assert_eq!(status.bound_port, Some(port));
        svc.shutdown().await;
    }

    #[tokio::test]
    async fn listener_handles_unknown_info_hash() {
        let mut catalog = HashMap::new();
        catalog.insert([9u8; 20], Ok(faker_handle()));
        let lookup = Arc::new(StaticLookup { catalog });
        let mut svc = PeerListenerService::new();
        svc.start(lookup);
        let port = free_port();
        svc.set_desired_port(Some(port));
        tokio::time::sleep(Duration::from_millis(300)).await;

        let stream = TcpStream::connect(("127.0.0.1", port)).await;
        assert!(stream.is_ok());
        let mut stream = stream.unwrap_or_else(|_| unreachable!());
        let hs = PeerHandshake::new([0u8; 8], [1u8; 20], *b"-UT0001-123456789012").to_bytes();
        let sent = stream.write_all(&hs).await;
        assert!(sent.is_ok());

        let mut buf = [0u8; 68];
        let read = tokio::time::timeout(Duration::from_secs(1), stream.read(&mut buf)).await;
        assert!(read.is_ok());
        assert_eq!(read.unwrap_or_else(|_| unreachable!()).unwrap_or(1), 0);

        svc.shutdown().await;
    }

    #[tokio::test]
    async fn listener_rebinds_when_desired_port_changes() {
        let mut svc = PeerListenerService::new();
        let lookup = Arc::new(StaticLookup { catalog: HashMap::new() });
        svc.start(lookup);
        let mut status_rx = svc.subscribe();
        let port_a = free_port();
        let port_b = free_port();

        svc.set_desired_port(Some(port_a));
        let first = tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                if status_rx.changed().await.is_err() {
                    break svc.status();
                }
                let status = status_rx.borrow().clone();
                if status.bound_port == Some(port_a) {
                    break status;
                }
            }
        })
        .await
        .unwrap_or_else(|_| unreachable!());
        assert_eq!(first.bound_port, Some(port_a));

        svc.set_desired_port(Some(port_b));
        let second = tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                if status_rx.changed().await.is_err() {
                    break svc.status();
                }
                let status = status_rx.borrow().clone();
                if status.bound_port == Some(port_b) {
                    break status;
                }
            }
        })
        .await
        .unwrap_or_else(|_| unreachable!());
        assert_eq!(second.bound_port, Some(port_b));

        svc.shutdown().await;
    }

    #[tokio::test]
    async fn peer_id_bytes_uses_string_accessor() {
        let arr = peer_id_to_array("-UT0001-123456789012");
        assert!(arr.is_ok());
        assert_eq!(arr.unwrap_or_else(|_| unreachable!()).len(), 20);
    }

    #[tokio::test]
    async fn listener_retries_bind_after_port_becomes_available() {
        let hold = std::net::TcpListener::bind(("127.0.0.1", 0));
        assert!(hold.is_ok());
        let hold = hold.unwrap_or_else(|_| unreachable!());
        let port = hold.local_addr().unwrap_or_else(|_| unreachable!()).port();

        let mut svc = PeerListenerService::new();
        let lookup = Arc::new(StaticLookup { catalog: HashMap::new() });
        svc.start(lookup);
        let mut status_rx = svc.subscribe();

        svc.set_desired_port(Some(port));
        let failed = tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                if status_rx.changed().await.is_err() {
                    break svc.status();
                }
                let status = status_rx.borrow().clone();
                if status.bound_port.is_none() && status.last_error.is_some() {
                    break status;
                }
            }
        })
        .await
        .unwrap_or_else(|_| unreachable!());
        assert_eq!(failed.desired_port, Some(port));
        assert_eq!(failed.bound_port, None);
        assert!(failed.last_error.is_some());

        drop(hold);

        let recovered = tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                if status_rx.changed().await.is_err() {
                    break svc.status();
                }
                let status = status_rx.borrow().clone();
                if status.bound_port == Some(port) {
                    break status;
                }
            }
        })
        .await
        .unwrap_or_else(|_| unreachable!());
        assert_eq!(recovered.bound_port, Some(port));
        assert_eq!(recovered.last_error, None);

        svc.shutdown().await;
    }

    #[tokio::test]
    async fn duplicate_info_hash_is_reported_as_conflict() {
        let mut catalog = HashMap::new();
        catalog.insert([7u8; 20], Err("duplicate active instances for info_hash".to_string()));
        let mut svc = PeerListenerService::new();
        let lookup = Arc::new(StaticLookup { catalog });
        svc.start(lookup);
        let port = free_port();
        svc.set_desired_port(Some(port));
        tokio::time::sleep(Duration::from_millis(300)).await;

        let stream = TcpStream::connect(("127.0.0.1", port)).await;
        assert!(stream.is_ok());
        let mut stream = stream.unwrap_or_else(|_| unreachable!());
        let hs = PeerHandshake::new([0u8; 8], [7u8; 20], *b"-UT0001-123456789012").to_bytes();
        let sent = stream.write_all(&hs).await;
        assert!(sent.is_ok());

        let mut buf = [0u8; 68];
        let read = tokio::time::timeout(Duration::from_secs(1), stream.read(&mut buf)).await;
        assert!(read.is_ok());
        assert_eq!(read.unwrap_or_else(|_| unreachable!()).unwrap_or(1), 0);

        svc.shutdown().await;
    }
}
