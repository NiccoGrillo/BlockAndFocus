//! Shared types for BlockAndFocus IPC protocol and configuration.

use chrono::{NaiveTime, Weekday};
use serde::{Deserialize, Serialize};

/// IPC Commands sent from the UI to the daemon.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum Command {
    /// Get current daemon status
    GetStatus,

    /// Get the current blocklist
    GetBlocklist,

    /// Add a domain to the blocklist
    AddDomain { domain: String },

    /// Remove a domain from the blocklist
    RemoveDomain { domain: String },

    /// Get the current schedule configuration
    GetSchedule,

    /// Update the schedule configuration
    UpdateSchedule { schedule: Schedule },

    /// Request a bypass (triggers quiz challenge)
    RequestBypass { duration_minutes: u32 },

    /// Submit quiz answers to complete bypass request
    SubmitQuizAnswers {
        challenge_id: String,
        answers: Vec<i32>,
    },

    /// Cancel an active bypass early
    CancelBypass,

    /// Ping to check if daemon is alive
    Ping,
}

/// IPC Responses sent from the daemon to the UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum Response {
    /// Current daemon status
    Status(Status),

    /// Current blocklist
    Blocklist { domains: Vec<String> },

    /// Current schedule configuration
    Schedule(Schedule),

    /// Quiz challenge for bypass request
    QuizChallenge(QuizChallenge),

    /// Operation completed successfully
    Success,

    /// Pong response to ping
    Pong,

    /// Error response
    Error { code: ErrorCode, message: String },
}

/// Current daemon status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Status {
    /// Whether blocking is currently active
    pub blocking_active: bool,

    /// Number of domains in the blocklist
    pub blocked_domains_count: usize,

    /// Number of DNS queries blocked since daemon start
    pub queries_blocked: u64,

    /// Number of DNS queries forwarded since daemon start
    pub queries_forwarded: u64,

    /// Unix timestamp when bypass expires (None if no active bypass)
    pub bypass_until: Option<i64>,

    /// Name of the currently active schedule rule (None if outside schedule)
    pub active_schedule_rule: Option<String>,

    /// Whether the schedule is enabled
    pub schedule_enabled: bool,
}

/// Quiz challenge for bypass requests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuizChallenge {
    /// Unique challenge ID
    pub challenge_id: String,

    /// Questions to display (e.g., "23 + 45 = ?")
    pub questions: Vec<String>,

    /// Unix timestamp when this challenge expires
    pub expires_at: i64,
}

/// Schedule configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Schedule {
    /// Whether scheduling is enabled
    pub enabled: bool,

    /// List of schedule rules
    pub rules: Vec<ScheduleRule>,
}

/// A single schedule rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleRule {
    /// Human-readable name for this rule
    pub name: String,

    /// Days of the week this rule applies
    pub days: Vec<WeekdayWrapper>,

    /// Start time (blocking begins)
    pub start_time: NaiveTimeWrapper,

    /// End time (blocking ends)
    pub end_time: NaiveTimeWrapper,
}

/// Wrapper for chrono::Weekday with serde support.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WeekdayWrapper {
    Mon,
    Tue,
    Wed,
    Thu,
    Fri,
    Sat,
    Sun,
}

impl From<WeekdayWrapper> for Weekday {
    fn from(w: WeekdayWrapper) -> Self {
        match w {
            WeekdayWrapper::Mon => Weekday::Mon,
            WeekdayWrapper::Tue => Weekday::Tue,
            WeekdayWrapper::Wed => Weekday::Wed,
            WeekdayWrapper::Thu => Weekday::Thu,
            WeekdayWrapper::Fri => Weekday::Fri,
            WeekdayWrapper::Sat => Weekday::Sat,
            WeekdayWrapper::Sun => Weekday::Sun,
        }
    }
}

impl From<Weekday> for WeekdayWrapper {
    fn from(w: Weekday) -> Self {
        match w {
            Weekday::Mon => WeekdayWrapper::Mon,
            Weekday::Tue => WeekdayWrapper::Tue,
            Weekday::Wed => WeekdayWrapper::Wed,
            Weekday::Thu => WeekdayWrapper::Thu,
            Weekday::Fri => WeekdayWrapper::Fri,
            Weekday::Sat => WeekdayWrapper::Sat,
            Weekday::Sun => WeekdayWrapper::Sun,
        }
    }
}

/// Wrapper for NaiveTime with string serialization (HH:MM format).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NaiveTimeWrapper(pub NaiveTime);

impl Serialize for NaiveTimeWrapper {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let s = self.0.format("%H:%M").to_string();
        serializer.serialize_str(&s)
    }
}

impl<'de> Deserialize<'de> for NaiveTimeWrapper {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        NaiveTime::parse_from_str(&s, "%H:%M")
            .map(NaiveTimeWrapper)
            .map_err(serde::de::Error::custom)
    }
}

impl From<NaiveTime> for NaiveTimeWrapper {
    fn from(t: NaiveTime) -> Self {
        NaiveTimeWrapper(t)
    }
}

impl From<NaiveTimeWrapper> for NaiveTime {
    fn from(t: NaiveTimeWrapper) -> Self {
        t.0
    }
}

/// Error codes for IPC responses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    /// Invalid command format
    InvalidCommand,

    /// Domain is invalid or already exists/doesn't exist
    InvalidDomain,

    /// Quiz challenge not found or expired
    QuizNotFound,

    /// Quiz challenge has expired
    QuizExpired,

    /// Wrong quiz answers
    QuizFailed,

    /// Quiz solved too quickly (anti-automation)
    QuizTooFast,

    /// Cannot bypass during strict schedule
    BypassNotAllowed,

    /// Configuration error
    ConfigError,

    /// Internal daemon error
    InternalError,
}

/// Configuration file structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub dns: DnsConfig,
    pub blocking: BlockingConfig,
    pub schedule: Schedule,
    pub quiz: QuizConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            dns: DnsConfig::default(),
            blocking: BlockingConfig::default(),
            schedule: Schedule::default(),
            quiz: QuizConfig::default(),
        }
    }
}

/// DNS server configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsConfig {
    /// Upstream DNS servers
    pub upstream: Vec<String>,

    /// Address to listen on
    pub listen_address: String,

    /// Port to listen on
    pub listen_port: u16,
}

impl Default for DnsConfig {
    fn default() -> Self {
        Self {
            upstream: vec!["1.1.1.1".to_string(), "8.8.8.8".to_string()],
            listen_address: "127.0.0.1".to_string(),
            listen_port: 53,
        }
    }
}

/// Blocking configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockingConfig {
    /// Whether blocking is enabled
    pub enabled: bool,

    /// List of blocked domains
    pub domains: Vec<String>,
}

impl Default for BlockingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            domains: vec![
                "facebook.com".to_string(),
                "twitter.com".to_string(),
                "instagram.com".to_string(),
                "reddit.com".to_string(),
                "tiktok.com".to_string(),
            ],
        }
    }
}

/// Quiz configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuizConfig {
    /// Number of questions per quiz
    pub num_questions: u32,

    /// Minimum operand value
    pub min_operand: i32,

    /// Maximum operand value
    pub max_operand: i32,

    /// Quiz timeout in seconds
    pub timeout_seconds: u32,

    /// Minimum time to solve (anti-automation)
    pub min_solve_seconds: u32,
}

impl Default for QuizConfig {
    fn default() -> Self {
        Self {
            num_questions: 3,
            min_operand: 10,
            max_operand: 99,
            timeout_seconds: 60,
            min_solve_seconds: 3,
        }
    }
}

/// Socket path for IPC.
pub const IPC_SOCKET_PATH: &str = "/var/run/blockandfocus.sock";

/// Development socket path (for non-root testing).
pub const IPC_SOCKET_PATH_DEV: &str = "/tmp/blockandfocus-dev.sock";

/// Config file path.
pub const CONFIG_PATH: &str = "/Library/Application Support/BlockAndFocus/config.toml";

/// Development config path.
pub const CONFIG_PATH_DEV: &str = "./config.toml";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_serialization() {
        let cmd = Command::AddDomain {
            domain: "facebook.com".to_string(),
        };
        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("AddDomain"));
        assert!(json.contains("facebook.com"));

        let parsed: Command = serde_json::from_str(&json).unwrap();
        match parsed {
            Command::AddDomain { domain } => assert_eq!(domain, "facebook.com"),
            _ => panic!("Wrong command type"),
        }
    }

    #[test]
    fn test_response_serialization() {
        let resp = Response::Status(Status {
            blocking_active: true,
            blocked_domains_count: 5,
            queries_blocked: 100,
            queries_forwarded: 500,
            bypass_until: None,
            active_schedule_rule: Some("Work Hours".to_string()),
            schedule_enabled: true,
        });
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("Status"));
        assert!(json.contains("blocking_active"));
    }

    #[test]
    fn test_time_wrapper_serialization() {
        let time = NaiveTimeWrapper(NaiveTime::from_hms_opt(9, 30, 0).unwrap());
        let json = serde_json::to_string(&time).unwrap();
        assert_eq!(json, "\"09:30\"");

        let parsed: NaiveTimeWrapper = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.0.hour(), 9);
        assert_eq!(parsed.0.minute(), 30);
    }
}
