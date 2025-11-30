//! Unix domain socket IPC server.

use crate::AppState;
use anyhow::{Context, Result};
use blockandfocus_shared::{
    Command, ErrorCode, Response, Status, IPC_SOCKET_PATH, IPC_SOCKET_PATH_DEV,
};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// IPC server for handling UI commands.
pub struct IpcServer;

impl IpcServer {
    /// Run the IPC server.
    pub async fn run(state: Arc<RwLock<AppState>>) -> Result<()> {
        let is_dev = std::env::var("BLOCKANDFOCUS_DEV").is_ok();
        let socket_path = if is_dev {
            IPC_SOCKET_PATH_DEV
        } else {
            IPC_SOCKET_PATH
        };

        // Remove existing socket file if present
        let _ = std::fs::remove_file(socket_path);

        // Create parent directory if needed
        if let Some(parent) = std::path::Path::new(socket_path).parent() {
            std::fs::create_dir_all(parent).ok();
        }

        info!("Starting IPC server on {}", socket_path);

        let listener = UnixListener::bind(socket_path)
            .with_context(|| format!("Failed to bind IPC socket: {}", socket_path))?;

        // Set socket permissions (readable/writable by owner and group)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o660);
            std::fs::set_permissions(socket_path, perms).ok();
        }

        info!("IPC server listening on {}", socket_path);

        loop {
            match listener.accept().await {
                Ok((stream, _addr)) => {
                    let state_clone = state.clone();
                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_connection(stream, state_clone).await {
                            warn!("IPC connection error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    error!("Failed to accept IPC connection: {}", e);
                }
            }
        }
    }

    /// Handle a single IPC connection.
    async fn handle_connection(
        stream: UnixStream,
        state: Arc<RwLock<AppState>>,
    ) -> Result<()> {
        let (reader, mut writer) = stream.into_split();
        let mut reader = BufReader::new(reader);
        let mut line = String::new();

        loop {
            line.clear();
            let bytes_read = reader.read_line(&mut line).await?;

            if bytes_read == 0 {
                // Connection closed
                break;
            }

            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            debug!(command = %trimmed, "Received IPC command");

            let response = match serde_json::from_str::<Command>(trimmed) {
                Ok(cmd) => Self::handle_command(cmd, &state).await,
                Err(e) => {
                    warn!("Invalid IPC command: {}", e);
                    Response::Error {
                        code: ErrorCode::InvalidCommand,
                        message: format!("Invalid command: {}", e),
                    }
                }
            };

            let response_json = serde_json::to_string(&response)?;
            writer.write_all(response_json.as_bytes()).await?;
            writer.write_all(b"\n").await?;
            writer.flush().await?;
        }

        Ok(())
    }

    /// Handle a single IPC command.
    async fn handle_command(cmd: Command, state: &Arc<RwLock<AppState>>) -> Response {
        match cmd {
            Command::Ping => Response::Pong,

            Command::GetStatus => {
                let state_guard = state.read().await;
                let config = state_guard.config.get();

                Response::Status(Status {
                    blocking_active: state_guard.is_blocking_active(),
                    blocked_domains_count: config.blocking.domains.len(),
                    queries_blocked: state_guard.stats.queries_blocked,
                    queries_forwarded: state_guard.stats.queries_forwarded,
                    bypass_until: state_guard.bypass_until,
                    active_schedule_rule: state_guard.schedule.active_rule_name(),
                    schedule_enabled: config.schedule.enabled,
                })
            }

            Command::GetBlocklist => {
                let state_guard = state.read().await;
                let domains = state_guard.config.blocked_domains();
                Response::Blocklist { domains }
            }

            Command::AddDomain { domain } => {
                let mut state_guard = state.write().await;
                match state_guard.config.add_domain(domain.clone()).await {
                    Ok(()) => {
                        // Update the blocker with new domain list
                        let domains = state_guard.config.blocked_domains();
                        state_guard.blocker.update_domains(domains);
                        info!(domain = %domain, "Domain added to blocklist");
                        Response::Success
                    }
                    Err(e) => Response::Error {
                        code: ErrorCode::ConfigError,
                        message: format!("Failed to add domain: {}", e),
                    },
                }
            }

            Command::RemoveDomain { domain } => {
                let mut state_guard = state.write().await;
                match state_guard.config.remove_domain(&domain).await {
                    Ok(true) => {
                        // Update the blocker with new domain list
                        let domains = state_guard.config.blocked_domains();
                        state_guard.blocker.update_domains(domains);
                        info!(domain = %domain, "Domain removed from blocklist");
                        Response::Success
                    }
                    Ok(false) => Response::Error {
                        code: ErrorCode::InvalidDomain,
                        message: "Domain not found in blocklist".to_string(),
                    },
                    Err(e) => Response::Error {
                        code: ErrorCode::ConfigError,
                        message: format!("Failed to remove domain: {}", e),
                    },
                }
            }

            Command::GetSchedule => {
                let state_guard = state.read().await;
                let schedule = state_guard.config.get().schedule.clone();
                Response::Schedule(schedule)
            }

            Command::UpdateSchedule { schedule } => {
                let mut state_guard = state.write().await;

                // Update schedule engine
                state_guard.schedule.update(schedule.clone());

                // Persist to config
                match state_guard.config.update(|c| c.schedule = schedule).await {
                    Ok(()) => {
                        info!("Schedule updated");
                        Response::Success
                    }
                    Err(e) => Response::Error {
                        code: ErrorCode::ConfigError,
                        message: format!("Failed to update schedule: {}", e),
                    },
                }
            }

            Command::RequestBypass { duration_minutes } => {
                let mut state_guard = state.write().await;
                let challenge = state_guard.quiz.generate_challenge();

                // Store the requested duration for when quiz is validated
                // (We'll need to pass it through somehow - for now, store in challenge metadata)
                debug!(
                    duration_minutes,
                    challenge_id = %challenge.challenge_id,
                    "Bypass requested, quiz generated"
                );

                Response::QuizChallenge(challenge)
            }

            Command::SubmitQuizAnswers {
                challenge_id,
                answers,
            } => {
                let mut state_guard = state.write().await;

                match state_guard.quiz.validate_answers(&challenge_id, &answers) {
                    Ok(()) => {
                        // Quiz passed, activate bypass
                        // Default to 15 minutes if not specified
                        // In a real implementation, we'd store the duration with the challenge
                        state_guard.activate_bypass(15);
                        info!("Quiz validated, bypass activated");
                        Response::Success
                    }
                    Err(e) => {
                        let code = match e {
                            crate::quiz::QuizError::NotFound => ErrorCode::QuizNotFound,
                            crate::quiz::QuizError::Expired => ErrorCode::QuizExpired,
                            crate::quiz::QuizError::TooFast => ErrorCode::QuizTooFast,
                            crate::quiz::QuizError::WrongAnswerCount
                            | crate::quiz::QuizError::WrongAnswer => ErrorCode::QuizFailed,
                        };
                        Response::Error {
                            code,
                            message: e.to_string(),
                        }
                    }
                }
            }

            Command::CancelBypass => {
                let mut state_guard = state.write().await;
                state_guard.cancel_bypass();
                Response::Success
            }
        }
    }
}
