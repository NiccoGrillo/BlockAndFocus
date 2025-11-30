//! Tauri commands for UI-daemon communication

use blockandfocus_shared::{Response, Schedule};
use crate::{AppState, StatusInfo, QuizInfo, QuizResult};
use tauri::State;

/// Get the current daemon status
#[tauri::command]
pub async fn get_status(state: State<'_, AppState>) -> Result<StatusInfo, String> {
    let client = state.client.lock().await;

    if !client.is_daemon_running() {
        return Ok(StatusInfo {
            blocking_active: false,
            schedule_enabled: false,
            schedule_active: false,
            bypass_active: false,
            bypass_remaining_seconds: None,
            blocked_count: 0,
            daemon_connected: false,
        });
    }

    match client.get_status().await {
        Ok(Response::Status(status)) => {
            let now = chrono::Utc::now().timestamp();
            let bypass_remaining = status.bypass_until.map(|until| (until - now).max(0));

            Ok(StatusInfo {
                blocking_active: status.blocking_active,
                schedule_enabled: status.schedule_enabled,
                schedule_active: status.active_schedule_rule.is_some(),
                bypass_active: status.bypass_until.is_some() && status.bypass_until.unwrap() > now,
                bypass_remaining_seconds: bypass_remaining,
                blocked_count: status.queries_blocked,
                daemon_connected: true,
            })
        }
        Ok(Response::Error { message, .. }) => Err(message),
        Ok(_) => Err("Unexpected response from daemon".to_string()),
        Err(e) => Err(format!("Failed to get status: {}", e)),
    }
}

/// Get the current blocklist
#[tauri::command]
pub async fn get_blocklist(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let client = state.client.lock().await;

    match client.get_blocklist().await {
        Ok(Response::Blocklist { domains }) => Ok(domains),
        Ok(Response::Error { message, .. }) => Err(message),
        Ok(_) => Err("Unexpected response from daemon".to_string()),
        Err(e) => Err(format!("Failed to get blocklist: {}", e)),
    }
}

/// Add a domain to the blocklist
#[tauri::command]
pub async fn add_domain(state: State<'_, AppState>, domain: String) -> Result<bool, String> {
    let client = state.client.lock().await;

    match client.add_domain(domain).await {
        Ok(Response::Success) => Ok(true),
        Ok(Response::Error { message, .. }) => Err(message),
        Ok(_) => Err("Unexpected response from daemon".to_string()),
        Err(e) => Err(format!("Failed to add domain: {}", e)),
    }
}

/// Remove a domain from the blocklist
#[tauri::command]
pub async fn remove_domain(state: State<'_, AppState>, domain: String) -> Result<bool, String> {
    let client = state.client.lock().await;

    match client.remove_domain(domain).await {
        Ok(Response::Success) => Ok(true),
        Ok(Response::Error { message, .. }) => Err(message),
        Ok(_) => Err("Unexpected response from daemon".to_string()),
        Err(e) => Err(format!("Failed to remove domain: {}", e)),
    }
}

/// Get the current schedule
#[tauri::command]
pub async fn get_schedule(state: State<'_, AppState>) -> Result<Schedule, String> {
    let client = state.client.lock().await;

    match client.get_schedule().await {
        Ok(Response::Schedule(schedule)) => Ok(schedule),
        Ok(Response::Error { message, .. }) => Err(message),
        Ok(_) => Err("Unexpected response from daemon".to_string()),
        Err(e) => Err(format!("Failed to get schedule: {}", e)),
    }
}

/// Set schedule enabled status
#[tauri::command]
pub async fn set_schedule_enabled(
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<bool, String> {
    let client = state.client.lock().await;

    // First get the current schedule
    let schedule = match client.get_schedule().await {
        Ok(Response::Schedule(s)) => s,
        Ok(Response::Error { message, .. }) => return Err(message),
        Ok(_) => return Err("Unexpected response from daemon".to_string()),
        Err(e) => return Err(format!("Failed to get schedule: {}", e)),
    };

    // Update the enabled flag
    let updated_schedule = Schedule {
        enabled,
        rules: schedule.rules,
    };

    match client.update_schedule(updated_schedule).await {
        Ok(Response::Success) => Ok(true),
        Ok(Response::Error { message, .. }) => Err(message),
        Ok(_) => Err("Unexpected response from daemon".to_string()),
        Err(e) => Err(format!("Failed to update schedule: {}", e)),
    }
}

/// Request a bypass quiz
#[tauri::command]
pub async fn request_bypass(
    state: State<'_, AppState>,
    duration_minutes: u32,
) -> Result<QuizInfo, String> {
    let client = state.client.lock().await;

    match client.request_bypass(duration_minutes).await {
        Ok(Response::QuizChallenge(quiz)) => Ok(QuizInfo {
            challenge_id: quiz.challenge_id,
            questions: quiz.questions,
            expires_at: quiz.expires_at,
        }),
        Ok(Response::Error { message, .. }) => Err(message),
        Ok(_) => Err("Unexpected response from daemon".to_string()),
        Err(e) => Err(format!("Failed to request bypass: {}", e)),
    }
}

/// Submit quiz answers
#[tauri::command]
pub async fn submit_quiz_answers(
    state: State<'_, AppState>,
    challenge_id: String,
    answers: Vec<i32>,
) -> Result<QuizResult, String> {
    let client = state.client.lock().await;

    match client.submit_quiz_answers(challenge_id, answers).await {
        Ok(Response::Success) => Ok(QuizResult {
            success: true,
            message: "Bypass granted!".to_string(),
        }),
        Ok(Response::Error { message, .. }) => Ok(QuizResult {
            success: false,
            message,
        }),
        Ok(_) => Err("Unexpected response from daemon".to_string()),
        Err(e) => Err(format!("Failed to submit answers: {}", e)),
    }
}

/// Cancel an active bypass
#[tauri::command]
pub async fn cancel_bypass(state: State<'_, AppState>) -> Result<bool, String> {
    let client = state.client.lock().await;

    match client.cancel_bypass().await {
        Ok(Response::Success) => Ok(true),
        Ok(Response::Error { message, .. }) => Err(message),
        Ok(_) => Err("Unexpected response from daemon".to_string()),
        Err(e) => Err(format!("Failed to cancel bypass: {}", e)),
    }
}
