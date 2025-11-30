//! Quiz generation and validation engine.

use blockandfocus_shared::{QuizChallenge, QuizConfig};
use chrono::Utc;
use rand::Rng;
use std::collections::HashMap;
use std::time::Instant;
use tracing::{debug, warn};
use uuid::Uuid;

/// Arithmetic operation for quiz questions.
#[derive(Debug, Clone, Copy)]
enum Operation {
    Add,
    Subtract,
    Multiply,
}

/// Internal question representation with answer.
#[derive(Debug, Clone)]
struct Question {
    display: String,
    answer: i32,
}

/// Pending quiz challenge waiting for answers.
#[derive(Debug)]
struct PendingChallenge {
    questions: Vec<Question>,
    created_at: Instant,
    expires_at: i64,
}

/// Quiz engine for generating and validating arithmetic challenges.
pub struct QuizEngine {
    config: QuizConfig,
    pending: HashMap<String, PendingChallenge>,
}

impl QuizEngine {
    /// Create a new quiz engine.
    pub fn new(config: QuizConfig) -> Self {
        Self {
            config,
            pending: HashMap::new(),
        }
    }

    /// Update the quiz configuration.
    pub fn update_config(&mut self, config: QuizConfig) {
        self.config = config;
    }

    /// Generate a new quiz challenge.
    pub fn generate_challenge(&mut self) -> QuizChallenge {
        // Clean up expired challenges first
        self.cleanup_expired();

        let challenge_id = Uuid::new_v4().to_string();
        let mut rng = rand::thread_rng();

        let questions: Vec<Question> = (0..self.config.num_questions)
            .map(|_| self.generate_question(&mut rng))
            .collect();

        let expires_at = Utc::now().timestamp() + self.config.timeout_seconds as i64;

        let challenge = QuizChallenge {
            challenge_id: challenge_id.clone(),
            questions: questions.iter().map(|q| q.display.clone()).collect(),
            expires_at,
        };

        // Store the pending challenge
        self.pending.insert(
            challenge_id,
            PendingChallenge {
                questions,
                created_at: Instant::now(),
                expires_at,
            },
        );

        debug!(
            num_questions = self.config.num_questions,
            expires_in = self.config.timeout_seconds,
            "Generated quiz challenge"
        );

        challenge
    }

    /// Validate quiz answers.
    ///
    /// Returns Ok(()) if all answers are correct, Err with reason otherwise.
    pub fn validate_answers(
        &mut self,
        challenge_id: &str,
        answers: &[i32],
    ) -> Result<(), QuizError> {
        // Get and remove the challenge (one-time use)
        let challenge = self
            .pending
            .remove(challenge_id)
            .ok_or(QuizError::NotFound)?;

        // Check expiry
        let now = Utc::now().timestamp();
        if now > challenge.expires_at {
            return Err(QuizError::Expired);
        }

        // Check minimum solve time (anti-automation)
        let solve_time = challenge.created_at.elapsed();
        if solve_time.as_secs() < self.config.min_solve_seconds as u64 {
            warn!(
                solve_time_secs = solve_time.as_secs(),
                min_required = self.config.min_solve_seconds,
                "Quiz solved suspiciously fast"
            );
            return Err(QuizError::TooFast);
        }

        // Check answer count
        if answers.len() != challenge.questions.len() {
            return Err(QuizError::WrongAnswerCount);
        }

        // Verify each answer
        for (i, (question, answer)) in challenge.questions.iter().zip(answers).enumerate() {
            if question.answer != *answer {
                debug!(
                    question_index = i,
                    expected = question.answer,
                    got = answer,
                    "Wrong answer"
                );
                return Err(QuizError::WrongAnswer);
            }
        }

        debug!("Quiz validated successfully");
        Ok(())
    }

    /// Generate a single arithmetic question.
    fn generate_question(&self, rng: &mut impl Rng) -> Question {
        let op = match rng.gen_range(0..3) {
            0 => Operation::Add,
            1 => Operation::Subtract,
            _ => Operation::Multiply,
        };

        let (a, b, answer, display) = match op {
            Operation::Add => {
                let a = rng.gen_range(self.config.min_operand..=self.config.max_operand);
                let b = rng.gen_range(self.config.min_operand..=self.config.max_operand);
                (a, b, a + b, format!("{} + {} = ?", a, b))
            }
            Operation::Subtract => {
                // Ensure positive result
                let a = rng.gen_range(self.config.min_operand..=self.config.max_operand);
                let b = rng.gen_range(self.config.min_operand..=a);
                (a, b, a - b, format!("{} - {} = ?", a, b))
            }
            Operation::Multiply => {
                // Use smaller numbers for multiplication
                let max = ((self.config.max_operand as f64).sqrt() as i32).max(12);
                let min = 2;
                let a = rng.gen_range(min..=max);
                let b = rng.gen_range(min..=max);
                (a, b, a * b, format!("{} × {} = ?", a, b))
            }
        };

        Question { display, answer }
    }

    /// Remove expired challenges.
    fn cleanup_expired(&mut self) {
        let now = Utc::now().timestamp();
        self.pending.retain(|_, c| c.expires_at > now);
    }
}

/// Quiz validation errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuizError {
    /// Challenge not found
    NotFound,
    /// Challenge has expired
    Expired,
    /// Quiz was solved too quickly (possible automation)
    TooFast,
    /// Wrong number of answers provided
    WrongAnswerCount,
    /// One or more answers are incorrect
    WrongAnswer,
}

impl std::fmt::Display for QuizError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QuizError::NotFound => write!(f, "Quiz challenge not found"),
            QuizError::Expired => write!(f, "Quiz challenge has expired"),
            QuizError::TooFast => write!(f, "Quiz was solved too quickly"),
            QuizError::WrongAnswerCount => write!(f, "Wrong number of answers"),
            QuizError::WrongAnswer => write!(f, "One or more answers are incorrect"),
        }
    }
}

impl std::error::Error for QuizError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> QuizConfig {
        QuizConfig {
            num_questions: 3,
            min_operand: 1,
            max_operand: 10,
            timeout_seconds: 60,
            min_solve_seconds: 0, // Disable for tests
        }
    }

    #[test]
    fn test_generate_challenge() {
        let mut engine = QuizEngine::new(test_config());
        let challenge = engine.generate_challenge();

        assert!(!challenge.challenge_id.is_empty());
        assert_eq!(challenge.questions.len(), 3);
        assert!(challenge.expires_at > Utc::now().timestamp());
    }

    #[test]
    fn test_validate_correct_answers() {
        let mut engine = QuizEngine::new(test_config());
        let challenge = engine.generate_challenge();

        // Get the correct answers from the pending challenge
        let pending = engine.pending.get(&challenge.challenge_id).unwrap();
        let correct_answers: Vec<i32> = pending.questions.iter().map(|q| q.answer).collect();

        let result = engine.validate_answers(&challenge.challenge_id, &correct_answers);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_wrong_answers() {
        let mut engine = QuizEngine::new(test_config());
        let challenge = engine.generate_challenge();

        // Submit wrong answers
        let wrong_answers = vec![99999, 99999, 99999];
        let result = engine.validate_answers(&challenge.challenge_id, &wrong_answers);
        assert_eq!(result, Err(QuizError::WrongAnswer));
    }

    #[test]
    fn test_challenge_not_found() {
        let mut engine = QuizEngine::new(test_config());
        let result = engine.validate_answers("nonexistent", &[1, 2, 3]);
        assert_eq!(result, Err(QuizError::NotFound));
    }

    #[test]
    fn test_one_time_use() {
        let mut engine = QuizEngine::new(test_config());
        let challenge = engine.generate_challenge();

        let pending = engine.pending.get(&challenge.challenge_id).unwrap();
        let correct_answers: Vec<i32> = pending.questions.iter().map(|q| q.answer).collect();

        // First validation succeeds
        let result = engine.validate_answers(&challenge.challenge_id, &correct_answers);
        assert!(result.is_ok());

        // Second validation fails (challenge consumed)
        let result = engine.validate_answers(&challenge.challenge_id, &correct_answers);
        assert_eq!(result, Err(QuizError::NotFound));
    }
}
