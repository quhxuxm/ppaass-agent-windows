use std::net::SocketAddr;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;

use crate::config::AGENT_CONFIG;
use crate::error::AgentError;
use crate::transport::dispatcher::ClientTransportDispatcher;
use crate::transport::{ClientTransport, TRANSPORT_MONITOR_FILE_PREFIX};

use crate::trace;
use crate::trace::TraceSubscriber;
use tokio::net::{TcpListener, TcpStream};
use tracing::{debug, error, info};

#[derive(Debug)]
pub struct AgentServer {
    transport_number: Arc<AtomicU64>,
}

impl AgentServer {
    pub(crate) fn new() -> Self {
        Self {
            transport_number: Arc::new(AtomicU64::new(0)),
        }
    }
    async fn accept_client_connection(
        tcp_listener: &TcpListener,
    ) -> Result<(TcpStream, SocketAddr), AgentError> {
        let (client_tcp_stream, client_socket_address) = tcp_listener.accept().await?;
        client_tcp_stream.set_nodelay(true)?;
        Ok((client_tcp_stream, client_socket_address))
    }

    pub async fn start(&mut self) -> Result<(), AgentError> {
        let agent_server_bind_addr = if AGENT_CONFIG.get_ipv6() {
            format!("::1:{}", AGENT_CONFIG.get_port())
        } else {
            format!("0.0.0.0:{}", AGENT_CONFIG.get_port())
        };
        info!("Agent server start to serve request on address: {agent_server_bind_addr}.");
        let tcp_listener = TcpListener::bind(&agent_server_bind_addr).await?;
        let (transport_trace_subscriber, _transport_trace_guard) =
            trace::init_transport_tracing_subscriber(
                TRANSPORT_MONITOR_FILE_PREFIX,
                AGENT_CONFIG.get_transport_max_log_level(),
            )?;
        let transport_trace_subscriber = Arc::new(transport_trace_subscriber);
        loop {
            let (client_tcp_stream, client_socket_address) =
                match Self::accept_client_connection(&tcp_listener).await {
                    Ok(accept_result) => accept_result,
                    Err(e) => {
                        error!(
                            "Agent server fail to accept client connection because of error: {e:?}"
                        );
                        continue;
                    }
                };
            debug!(
                "Accept client tcp connection on address: {}",
                client_socket_address
            );

            Self::handle_client_connection(
                client_tcp_stream,
                client_socket_address,
                self.transport_number.clone().clone(),
                transport_trace_subscriber.clone(),
            );
        }
    }

    fn handle_client_connection(
        client_tcp_stream: TcpStream,
        client_socket_address: SocketAddr,
        transport_number: Arc<AtomicU64>,
        transport_trace_subscriber: Arc<TraceSubscriber>,
    ) {
        tokio::spawn(async move {
            let client_transport =
                ClientTransportDispatcher::dispatch(client_tcp_stream, client_socket_address)
                    .await?;
            match client_transport {
                ClientTransport::Socks5(socks5_transport) => {
                    socks5_transport
                        .process(transport_number.clone(), transport_trace_subscriber.clone())
                        .await?;
                }
                ClientTransport::Http(http_transport) => {
                    http_transport
                        .process(transport_number.clone(), transport_trace_subscriber.clone())
                        .await?;
                }
            };
            debug!("Client transport [{client_socket_address}] complete to serve.");
            Ok::<(), AgentError>(())
        });
    }
}
