//! Quiz validation utilities.
//!
//! This module is intentionally minimal - most validation logic is in the QuizEngine.
//! This file exists for potential future expansion (e.g., HMAC tokens, validation receipts).

// Re-export QuizError for convenience
pub use super::generator::QuizError;
