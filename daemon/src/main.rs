//! BlockAndFocus DNS Daemon
//!
//! A DNS-based domain blocker for productivity.

mod config;
mod dns;
mod ipc;
mod quiz;
mod schedule;

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

use crate::config::ConfigManager;
use crate::dns::{DnsServer, DomainBlocker};
use crate::ipc::IpcServer;
use crate::quiz::QuizEngine;
use crate::schedule::ScheduleEngine;

/// Shared application state.
pub struct AppState {
    pub config: ConfigManager,
    pub schedule: ScheduleEngine,
    pub quiz: QuizEngine,
    pub blocker: DomainBlocker,
    pub stats: Stats,
    pub bypass_until: Option<i64>,
}

/// Runtime statistics.
#[derive(Default)]
pub struct Stats {
    pub queries_blocked: u64,
    pub queries_forwarded: u64,
}

impl AppState {
    pub fn new(config: ConfigManager) -> Self {
        let cfg = config.get();
        let schedule_config = cfg.schedule.clone();
        let quiz_config = cfg.quiz.clone();
        let blocked_domains = cfg.blocking.domains.clone();

        Self {
            config,
            schedule: ScheduleEngine::new(schedule_config),
            quiz: QuizEngine::new(quiz_config),
            blocker: DomainBlocker::new(blocked_domains),
            stats: Stats::default(),
            bypass_until: None,
        }
    }

    /// Check if blocking is currently active.
    pub fn is_blocking_active(&self) -> bool {
        // Check if blocking is enabled in config
        if !self.config.get().blocking.enabled {
            return false;
        }

        // Check if there's an active bypass
        if let Some(bypass_until) = self.bypass_until {
            let now = chrono::Utc::now().timestamp();
            if now < bypass_until {
                return false;
            }
        }

        // Check schedule
        if self.config.get().schedule.enabled {
            return self.schedule.is_blocking_time();
        }

        true
    }

    /// Activate a bypass for the given duration.
    pub fn activate_bypass(&mut self, duration_minutes: u32) {
        let now = chrono::Utc::now().timestamp();
        self.bypass_until = Some(now + (duration_minutes as i64 * 60));
        info!(duration_minutes, "Bypass activated");
    }

    /// Cancel any active bypass.
    pub fn cancel_bypass(&mut self) {
        self.bypass_until = None;
        info!("Bypass cancelled");
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(Level::INFO.into()),
        )
        .init();

    info!("BlockAndFocus daemon starting...");

    // Check if running in development mode
    let is_dev = std::env::var("BLOCKANDFOCUS_DEV").is_ok();
    if is_dev {
        info!("Running in development mode");
    }

    // Load configuration
    let config = ConfigManager::load(is_dev)?;
    info!("Configuration loaded");

    // Create shared application state
    let state = Arc::new(RwLock::new(AppState::new(config)));

    // Start DNS server
    let dns_state = state.clone();
    let dns_handle = tokio::spawn(async move {
        if let Err(e) = DnsServer::run(dns_state).await {
            tracing::error!("DNS server error: {}", e);
        }
    });

    // Start IPC server
    let ipc_state = state.clone();
    let ipc_handle = tokio::spawn(async move {
        if let Err(e) = IpcServer::run(ipc_state).await {
            tracing::error!("IPC server error: {}", e);
        }
    });

    info!("BlockAndFocus daemon started successfully");

    // Wait for shutdown signal
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("Received shutdown signal");
        }
        _ = dns_handle => {
            tracing::error!("DNS server stopped unexpectedly");
        }
        _ = ipc_handle => {
            tracing::error!("IPC server stopped unexpectedly");
        }
    }

    info!("BlockAndFocus daemon shutting down");
    Ok(())
}
