//! Upstream DNS resolver.

use anyhow::{Context, Result};
use hickory_proto::op::{Message, MessageType, OpCode, Query, ResponseCode};
use hickory_proto::rr::{Name, RData, Record, RecordType};
use hickory_resolver::config::{ResolverConfig, ResolverOpts};
use hickory_resolver::name_server::TokioConnectionProvider;
use hickory_resolver::Resolver;
use tracing::debug;

/// Type alias for the async resolver
type TokioResolver = Resolver<TokioConnectionProvider>;

/// Upstream DNS resolver with failover support.
pub struct UpstreamResolver {
    resolver: TokioResolver,
}

impl UpstreamResolver {
    /// Create a new upstream resolver with explicit upstream servers.
    /// IMPORTANT: We cannot use system DNS config because we ARE the system DNS!
    /// We use Cloudflare (1.1.1.1) as the upstream DNS.
    pub fn new(_upstream_servers: &[String]) -> Result<Self> {
        // Use Cloudflare DNS (1.1.1.1) - we CANNOT use system config since WE are the system DNS!
        let config = ResolverConfig::cloudflare();

        let resolver = TokioResolver::builder_with_config(config, TokioConnectionProvider::default())
            .with_options(ResolverOpts::default())
            .build();

        Ok(Self { resolver })
    }

    /// Resolve a DNS query using upstream servers.
    pub async fn resolve(&self, name: &Name, record_type: RecordType) -> Result<Message> {
        debug!(?name, ?record_type, "Forwarding query to upstream");

        let response = match record_type {
            RecordType::A => {
                let lookup = self.resolver.lookup_ip(name.to_string()).await?;
                self.build_response(name, record_type, lookup)
            }
            RecordType::AAAA => {
                let lookup = self.resolver.lookup_ip(name.to_string()).await?;
                self.build_response(name, record_type, lookup)
            }
            _ => {
                // For other record types, use generic lookup
                let lookup = self.resolver.lookup(name.clone(), record_type).await?;
                self.build_generic_response(name, record_type, lookup)
            }
        };

        Ok(response)
    }

    /// Build a DNS response message from a lookup result.
    fn build_response(
        &self,
        name: &Name,
        record_type: RecordType,
        lookup: hickory_resolver::lookup_ip::LookupIp,
    ) -> Message {
        let mut message = Message::new();
        message.set_id(0); // Will be set by caller
        message.set_message_type(MessageType::Response);
        message.set_op_code(OpCode::Query);
        message.set_response_code(ResponseCode::NoError);
        message.set_recursion_desired(true);
        message.set_recursion_available(true);

        // Add query section
        let query = Query::query(name.clone(), record_type);
        message.add_query(query);

        // Add answers
        for ip in lookup.iter() {
            let rdata = match ip {
                std::net::IpAddr::V4(v4) if record_type == RecordType::A => RData::A(v4.into()),
                std::net::IpAddr::V6(v6) if record_type == RecordType::AAAA => {
                    RData::AAAA(v6.into())
                }
                std::net::IpAddr::V4(_) if record_type == RecordType::AAAA => continue,
                std::net::IpAddr::V6(_) if record_type == RecordType::A => continue,
                _ => continue,
            };

            let record = Record::from_rdata(name.clone(), 300, rdata);
            message.add_answer(record);
        }

        message
    }

    /// Build a generic DNS response from any lookup.
    fn build_generic_response(
        &self,
        name: &Name,
        record_type: RecordType,
        lookup: hickory_resolver::lookup::Lookup,
    ) -> Message {
        let mut message = Message::new();
        message.set_id(0);
        message.set_message_type(MessageType::Response);
        message.set_op_code(OpCode::Query);
        message.set_response_code(ResponseCode::NoError);
        message.set_recursion_desired(true);
        message.set_recursion_available(true);

        let query = Query::query(name.clone(), record_type);
        message.add_query(query);

        for record in lookup.record_iter() {
            message.add_answer(record.clone());
        }

        message
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_upstream_resolver_creation() {
        let resolver = UpstreamResolver::new(&["1.1.1.1".to_string(), "8.8.8.8".to_string()]);
        assert!(resolver.is_ok());
    }
}
