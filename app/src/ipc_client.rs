//! IPC Client for communicating with the BlockAndFocus daemon
//!
//! Uses Unix domain sockets to send commands and receive responses.

use anyhow::{Context, Result};
use blockandfocus_shared::{Command, Response, Schedule, IPC_SOCKET_PATH, IPC_SOCKET_PATH_DEV};
use std::path::Path;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

/// Client for communicating with the daemon over IPC
pub struct IpcClient {
    socket_path: String,
}

impl IpcClient {
    /// Create a new IPC client
    pub fn new() -> Self {
        // Use development socket path if running in dev mode
        let socket_path = if std::env::var("BLOCKANDFOCUS_DEV").is_ok() {
            IPC_SOCKET_PATH_DEV.to_string()
        } else {
            IPC_SOCKET_PATH.to_string()
        };

        Self { socket_path }
    }

    /// Check if the daemon is running (socket exists)
    pub fn is_daemon_running(&self) -> bool {
        Path::new(&self.socket_path).exists()
    }

    /// Send a command to the daemon and receive a response
    pub async fn send_command(&self, command: Command) -> Result<Response> {
        // Connect to the daemon
        let stream = UnixStream::connect(&self.socket_path)
            .await
            .context("Failed to connect to daemon. Is it running?")?;

        let (reader, mut writer) = stream.into_split();

        // Serialize and send the command
        let mut json = serde_json::to_string(&command)?;
        json.push('\n');
        writer.write_all(json.as_bytes()).await?;

        // Read the response
        let mut reader = BufReader::new(reader);
        let mut response_line = String::new();
        reader.read_line(&mut response_line).await?;

        // Parse the response
        let response: Response = serde_json::from_str(&response_line)
            .context("Failed to parse daemon response")?;

        Ok(response)
    }

    /// Get the current daemon status
    pub async fn get_status(&self) -> Result<Response> {
        self.send_command(Command::GetStatus).await
    }

    /// Get the current blocklist
    pub async fn get_blocklist(&self) -> Result<Response> {
        self.send_command(Command::GetBlocklist).await
    }

    /// Add a domain to the blocklist
    pub async fn add_domain(&self, domain: String) -> Result<Response> {
        self.send_command(Command::AddDomain { domain }).await
    }

    /// Remove a domain from the blocklist
    pub async fn remove_domain(&self, domain: String) -> Result<Response> {
        self.send_command(Command::RemoveDomain { domain }).await
    }

    /// Get the current schedule
    pub async fn get_schedule(&self) -> Result<Response> {
        self.send_command(Command::GetSchedule).await
    }

    /// Update the schedule
    pub async fn update_schedule(&self, schedule: Schedule) -> Result<Response> {
        self.send_command(Command::UpdateSchedule { schedule }).await
    }

    /// Request a bypass quiz
    pub async fn request_bypass(&self, duration_minutes: u32) -> Result<Response> {
        self.send_command(Command::RequestBypass { duration_minutes }).await
    }

    /// Submit quiz answers
    pub async fn submit_quiz_answers(&self, challenge_id: String, answers: Vec<i32>) -> Result<Response> {
        self.send_command(Command::SubmitQuizAnswers { challenge_id, answers }).await
    }

    /// Cancel an active bypass
    pub async fn cancel_bypass(&self) -> Result<Response> {
        self.send_command(Command::CancelBypass).await
    }
}

impl Default for IpcClient {
    fn default() -> Self {
        Self::new()
    }
}
