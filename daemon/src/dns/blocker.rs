//! Domain blocking logic.

use tracing::debug;

/// Domain blocker with exact and subdomain matching.
pub struct DomainBlocker {
    blocked_domains: Vec<String>,
}

impl DomainBlocker {
    /// Create a new blocker with the given domain list.
    pub fn new(domains: Vec<String>) -> Self {
        let blocked_domains: Vec<String> = domains
            .into_iter()
            .map(|d| normalize_domain(&d))
            .collect();

        Self { blocked_domains }
    }

    /// Update the blocked domains list.
    pub fn update_domains(&mut self, domains: Vec<String>) {
        self.blocked_domains = domains
            .into_iter()
            .map(|d| normalize_domain(&d))
            .collect();
    }

    /// Check if a domain should be blocked.
    ///
    /// Matches exact domain and all subdomains.
    /// E.g., blocking "facebook.com" also blocks "www.facebook.com" and "m.facebook.com".
    pub fn should_block(&self, query_domain: &str) -> bool {
        let normalized = normalize_domain(query_domain);

        for blocked in &self.blocked_domains {
            // Exact match
            if normalized == *blocked {
                debug!(domain = %normalized, "Blocked (exact match)");
                return true;
            }

            // Subdomain match: query ends with ".blocked_domain"
            if normalized.ends_with(&format!(".{}", blocked)) {
                debug!(domain = %normalized, blocked = %blocked, "Blocked (subdomain match)");
                return true;
            }
        }

        false
    }

    /// Get the number of blocked domains.
    pub fn blocked_count(&self) -> usize {
        self.blocked_domains.len()
    }
}

/// Normalize a domain name for comparison.
fn normalize_domain(domain: &str) -> String {
    domain
        .to_lowercase()
        .trim()
        .trim_end_matches('.')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_match() {
        let blocker = DomainBlocker::new(vec![
            "facebook.com".to_string(),
            "twitter.com".to_string(),
        ]);

        assert!(blocker.should_block("facebook.com"));
        assert!(blocker.should_block("FACEBOOK.COM"));
        assert!(blocker.should_block("facebook.com."));
        assert!(blocker.should_block("twitter.com"));
        assert!(!blocker.should_block("google.com"));
    }

    #[test]
    fn test_subdomain_match() {
        let blocker = DomainBlocker::new(vec!["facebook.com".to_string()]);

        assert!(blocker.should_block("www.facebook.com"));
        assert!(blocker.should_block("m.facebook.com"));
        assert!(blocker.should_block("api.facebook.com"));
        assert!(blocker.should_block("deep.sub.facebook.com"));

        // Should NOT match domains that just contain the string
        assert!(!blocker.should_block("notfacebook.com"));
        assert!(!blocker.should_block("facebook.com.evil.com"));
    }

    #[test]
    fn test_update_domains() {
        let mut blocker = DomainBlocker::new(vec!["facebook.com".to_string()]);

        assert!(blocker.should_block("facebook.com"));
        assert!(!blocker.should_block("twitter.com"));

        blocker.update_domains(vec!["twitter.com".to_string()]);

        assert!(!blocker.should_block("facebook.com"));
        assert!(blocker.should_block("twitter.com"));
    }

    #[test]
    fn test_blocked_count() {
        let blocker = DomainBlocker::new(vec![
            "facebook.com".to_string(),
            "twitter.com".to_string(),
            "instagram.com".to_string(),
        ]);

        assert_eq!(blocker.blocked_count(), 3);
    }
}
