//! DNS server implementation using UDP sockets directly.

use crate::AppState;
use anyhow::{Context, Result};
use hickory_proto::op::{Message, MessageType, OpCode, ResponseCode};
use hickory_proto::rr::{Name, RData, Record, RecordType};
use hickory_proto::serialize::binary::{BinDecodable, BinEncodable};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use super::upstream::UpstreamResolver;

/// DNS server that handles blocking and forwarding.
pub struct DnsServer;

impl DnsServer {
    /// Run the DNS server.
    pub async fn run(state: Arc<RwLock<AppState>>) -> Result<()> {
        let config = {
            let state_guard = state.read().await;
            state_guard.config.get()
        };

        let listen_addr = format!(
            "{}:{}",
            config.dns.listen_address, config.dns.listen_port
        );

        info!("Starting DNS server on {}", listen_addr);

        // Bind UDP socket
        let socket = Arc::new(
            UdpSocket::bind(&listen_addr)
                .await
                .with_context(|| format!("Failed to bind DNS socket on {}", listen_addr))?,
        );

        info!("DNS server listening on {}", listen_addr);

        // Initialize upstream resolver
        let upstream = Arc::new(
            UpstreamResolver::new(&config.dns.upstream)
                .context("Failed to create upstream resolver")?,
        );

        // Main receive loop
        let mut buf = vec![0u8; 512];

        loop {
            match socket.recv_from(&mut buf).await {
                Ok((len, src)) => {
                    let query_data = buf[..len].to_vec();
                    let socket_clone = socket.clone();
                    let state_clone = state.clone();
                    let upstream_clone = upstream.clone();

                    // Handle query in a separate task
                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_query(
                            query_data,
                            src,
                            socket_clone,
                            state_clone,
                            upstream_clone,
                        )
                        .await
                        {
                            warn!("Error handling DNS query from {}: {}", src, e);
                        }
                    });
                }
                Err(e) => {
                    error!("Error receiving DNS query: {}", e);
                }
            }
        }
    }

    /// Handle a single DNS query.
    async fn handle_query(
        query_data: Vec<u8>,
        src: SocketAddr,
        socket: Arc<UdpSocket>,
        state: Arc<RwLock<AppState>>,
        upstream: Arc<UpstreamResolver>,
    ) -> Result<()> {
        // Parse the DNS query
        let query = Message::from_bytes(&query_data)
            .context("Failed to parse DNS query")?;

        let query_id = query.id();

        // Get the first question (most DNS queries have exactly one)
        let question = match query.queries().first() {
            Some(q) => q,
            None => {
                warn!("DNS query with no questions from {}", src);
                return Ok(());
            }
        };

        let name = question.name();
        let record_type = question.query_type();

        debug!(
            id = query_id,
            name = %name,
            record_type = ?record_type,
            "Received DNS query"
        );

        // Check if blocking is active and if domain should be blocked
        let should_block = {
            let state_guard = state.read().await;
            if state_guard.is_blocking_active() {
                state_guard.blocker.should_block(&name.to_string())
            } else {
                false
            }
        };

        let response = if should_block {
            // Update stats
            {
                let mut state_guard = state.write().await;
                state_guard.stats.queries_blocked += 1;
            }

            info!(name = %name, "Blocking DNS query");
            Self::create_blocked_response(&query, name, record_type)
        } else {
            // Update stats
            {
                let mut state_guard = state.write().await;
                state_guard.stats.queries_forwarded += 1;
            }

            // Forward to upstream
            match upstream.resolve(name, record_type).await {
                Ok(mut response) => {
                    response.set_id(query_id);
                    response
                }
                Err(e) => {
                    warn!(name = %name, error = %e, "Upstream resolution failed");
                    Self::create_servfail_response(&query)
                }
            }
        };

        // Send response
        let response_bytes = response.to_bytes()
            .context("Failed to serialize DNS response")?;

        socket
            .send_to(&response_bytes, src)
            .await
            .context("Failed to send DNS response")?;

        Ok(())
    }

    /// Create a blocked response (NXDOMAIN or 0.0.0.0).
    fn create_blocked_response(query: &Message, name: &Name, record_type: RecordType) -> Message {
        let mut response = Message::new();
        response.set_id(query.id());
        response.set_message_type(MessageType::Response);
        response.set_op_code(OpCode::Query);
        response.set_authoritative(false);
        response.set_recursion_desired(query.recursion_desired());
        response.set_recursion_available(true);

        // Copy the query
        for q in query.queries() {
            response.add_query(q.clone());
        }

        // Return 0.0.0.0 for A records (makes the block more obvious)
        if record_type == RecordType::A {
            response.set_response_code(ResponseCode::NoError);
            let rdata = RData::A("0.0.0.0".parse().unwrap());
            let record = Record::from_rdata(name.clone(), 60, rdata);
            response.add_answer(record);
        } else if record_type == RecordType::AAAA {
            // Return :: for AAAA records
            response.set_response_code(ResponseCode::NoError);
            let rdata = RData::AAAA("::".parse().unwrap());
            let record = Record::from_rdata(name.clone(), 60, rdata);
            response.add_answer(record);
        } else {
            // NXDOMAIN for other record types
            response.set_response_code(ResponseCode::NXDomain);
        }

        response
    }

    /// Create a SERVFAIL response for upstream failures.
    fn create_servfail_response(query: &Message) -> Message {
        let mut response = Message::new();
        response.set_id(query.id());
        response.set_message_type(MessageType::Response);
        response.set_op_code(OpCode::Query);
        response.set_response_code(ResponseCode::ServFail);
        response.set_recursion_desired(query.recursion_desired());
        response.set_recursion_available(true);

        for q in query.queries() {
            response.add_query(q.clone());
        }

        response
    }
}
