use std::{
    cmp::max,
    error::Error,
    io,
    net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs, UdpSocket},
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc, Arc, OnceLock,
    },
    thread,
    time::{Duration, Instant},
};

use log::{as_debug, as_display};

use crate::scaffolding::Context;

const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);
const SLEEP_DURATION: Duration = Duration::from_millis(500);

type Handler<T> = fn(&mut T, &SocketAddr) -> Result<(), Box<dyn Error>>;

pub struct ShutdownSignal {
    once: Arc<OnceLock<OnceLock<()>>>,
}

impl ShutdownSignal {
    fn new() -> Self {
        Self {
            once: Arc::new(OnceLock::new()),
        }
    }

    pub fn set_as_ctrl_c_handler(&self) -> Result<(), ctrlc::Error> {
        let mut cloned = self.clone();
        ctrlc::set_handler(move || {
            if cloned.start_shutdown() {
                log::info!(
                    reason = "ctrl-c received";
                    "Shutting down"
                );
            } else {
                log::info!("Already shutting down");
            }
        })
    }

    pub fn sleep_until_shutdown(&self) {
        while !self.is_shutdown_complete() {
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }

    #[allow(dead_code)]
    pub fn sleep_until_shutdown_or_timeout(&mut self, timeout: Duration) -> bool {
        let stop_at = Instant::now() + timeout;
        while !self.is_shutdown_complete() && Instant::now() < stop_at {
            std::thread::sleep(max(timeout / 100, SLEEP_DURATION));
        }
        let shutdown_due_to_timeout = !self.is_shutdown_complete();
        self.start_shutdown();
        self.complete_shutdown();
        shutdown_due_to_timeout
    }

    pub fn is_shutdown_initiated(&self) -> bool {
        self.once.get().is_some()
    }

    pub fn is_shutdown_complete(&self) -> bool {
        self.once
            .get()
            .map(|inner| inner.get().is_some())
            .unwrap_or(false)
    }

    pub fn start_shutdown(&mut self) -> bool {
        self.once.set(OnceLock::new()).is_ok()
    }

    pub fn complete_shutdown(&mut self) -> Option<bool> {
        self.once.get().map(|inner| inner.set(()).is_ok())
    }
}

impl Clone for ShutdownSignal {
    fn clone(&self) -> Self {
        Self {
            once: self.once.clone(),
        }
    }
}

pub(crate) trait Server {
    type Listener: Send + 'static;
    type Stream: Send + 'static;

    fn serve(
        &self,
        ctx: &Context,
        handler: Handler<Self::Stream>,
    ) -> Result<ShutdownSignal, Box<dyn Error>> {
        let active_threads = Arc::new(AtomicUsize::new(0));
        let listener = Self::get_listener(ctx.bind_address.as_str())?;
        let shutdown_signal = ShutdownSignal::new();
        let mut shutdown_signal_clone = shutdown_signal.clone();
        let local_address = Self::get_address(&listener)?;

        log::info!(
            address = as_display!(local_address),
            pid = as_display!(std::process::id());
            "Listening"
        );

        thread::Builder::new()
            .name("server-controller".into())
            .spawn(move || {
                let (request_sender, request_receiver) = mpsc::channel::<(Self::Stream, SocketAddr)>();
                if thread::Builder::new()
                    .name("accept-and-forward".into())
                    .spawn(move || {
                        while let Ok(stream) = Self::get_stream(&listener) {
                            if request_sender.send((stream, local_address)).is_err() {
                                break;
                            }
                        }
                    }
                ).is_err() {
                    log::error!("Unable to spawn thread to accept connections");
                }

                while !shutdown_signal_clone.is_shutdown_initiated() {
                    match request_receiver.recv_timeout(SLEEP_DURATION) {
                        Ok((mut stream, remote_address)) => {
                            log::info!(
                                remote_address = as_display!(remote_address);
                                "Got a connection"
                            );
                            let active_threads_clone = active_threads.clone();
                            let request_id = format!(
                                "{}-{}",
                                remote_address.port(),
                                match remote_address.ip() {
                                    std::net::IpAddr::V4(ipv4) => format!("{}", Into::<u32>::into(ipv4)),
                                    std::net::IpAddr::V6(ipv6) => format!("{}", Into::<u128>::into(ipv6)),
                                }
                            );
                            let request_handler = move || {
                                active_threads_clone.fetch_add(1, Ordering::SeqCst);
                                if let Some(err) = handler(&mut stream, &remote_address).err() {
                                    log::error!(
                                        error = as_display!(err),
                                        other_threads = as_display!(active_threads_clone.fetch_sub(1, Ordering::SeqCst)),
                                        remote_address = as_display!(remote_address);
                                        "Request complete"
                                    );
                                } else {
                                    log::info!(
                                        other_threads = as_display!(active_threads_clone.fetch_sub(1, Ordering::SeqCst)),
                                        remote_address = as_display!(remote_address);
                                        "Request complete"
                                    );
                                }
                            };
                            if thread::Builder::new()
                                .name(format!("request-handler-{}", request_id))
                                .spawn(request_handler)
                                .is_err()
                            {
                                log::error!(
                                    other_threads = as_display!(active_threads.fetch_sub(1, Ordering::SeqCst)),
                                    remote_address = as_display!(remote_address);
                                    "Unable to spawn thread to handle request"
                                )
                            }
                        }
                        Err(mpsc::RecvTimeoutError::Timeout) => { }
                        Err(err) => {
                            log::error!(
                                error = as_display!(err);
                                "Request receiver error, shutting down"
                            );
                            shutdown_signal_clone.start_shutdown();
                        }
                    }
                }

                // shutdown time!

                let stop_at = Instant::now() + SHUTDOWN_TIMEOUT;
                log::info!(
                    shutdown_timeout = as_debug!(SHUTDOWN_TIMEOUT),
                    active_threads = as_display!(active_threads.load(Ordering::SeqCst));
                    "Shutdown signal received"
                );
                while Instant::now() < stop_at && active_threads.load(Ordering::SeqCst) > 0 {
                    std::thread::sleep(SLEEP_DURATION);
                }
                if active_threads.load(Ordering::SeqCst) > 0 {
                    log::warn!(
                        active_threads = as_display!(active_threads.load(Ordering::SeqCst)),
                        shutdown_timeout = as_debug!(SHUTDOWN_TIMEOUT),
                        reason = "shutdown timeout reached";
                        "Stopping controller despite active threads"
                    );
                }
                shutdown_signal_clone.complete_shutdown();
            })?;
        Ok(shutdown_signal)
    }

    fn get_listener<A: ToSocketAddrs>(bind_address: A) -> io::Result<Self::Listener>;

    fn get_stream(listener: &Self::Listener) -> io::Result<Self::Stream>;

    fn get_address(listener: &Self::Listener) -> io::Result<SocketAddr>;
}

pub(crate) struct TcpServer();

impl TcpServer {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

pub(crate) struct UdpServer();

impl UdpServer {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

impl Server for TcpServer {
    type Listener = TcpListener;
    type Stream = TcpStream;

    fn get_listener<A: ToSocketAddrs>(bind_address: A) -> io::Result<Self::Listener> {
        Self::Listener::bind(bind_address)
    }

    fn get_stream(listener: &Self::Listener) -> io::Result<Self::Stream> {
        listener.accept().map(|(stream, _)| stream)
    }

    fn get_address(listener: &Self::Listener) -> io::Result<SocketAddr> {
        listener.local_addr()
    }
}

impl Server for UdpServer {
    type Listener = UdpSocket;
    type Stream = UdpSocket;

    fn get_listener<A: ToSocketAddrs>(bind_address: A) -> io::Result<Self::Listener> {
        Self::Listener::bind(bind_address)
    }

    fn get_stream(listener: &Self::Listener) -> io::Result<Self::Stream> {
        listener.try_clone()
    }

    fn get_address(listener: &Self::Listener) -> io::Result<SocketAddr> {
        listener.local_addr()
    }
}
