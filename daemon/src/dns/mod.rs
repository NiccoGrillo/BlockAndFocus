//! DNS server implementation.

mod blocker;
mod server;
mod upstream;

pub use blocker::DomainBlocker;
pub use server::DnsServer;
pub use upstream::UpstreamResolver;
