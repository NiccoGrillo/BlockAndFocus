//! Configuration loading and management.

use anyhow::{Context, Result};
use blockandfocus_shared::{Config, CONFIG_PATH, CONFIG_PATH_DEV};
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// Configuration manager with hot-reload support.
pub struct ConfigManager {
    config: Arc<RwLock<Config>>,
    path: String,
}

impl ConfigManager {
    /// Load configuration from file, or create default if not exists.
    pub fn load(is_dev: bool) -> Result<Self> {
        let path = if is_dev {
            CONFIG_PATH_DEV.to_string()
        } else {
            CONFIG_PATH.to_string()
        };

        let config = if Path::new(&path).exists() {
            info!("Loading config from {}", path);
            let content = fs::read_to_string(&path)
                .with_context(|| format!("Failed to read config file: {}", path))?;
            toml::from_str(&content)
                .with_context(|| format!("Failed to parse config file: {}", path))?
        } else {
            warn!("Config file not found at {}, using defaults", path);
            let config = Config::default();

            // Try to create the config file with defaults
            if let Err(e) = Self::save_config(&path, &config) {
                warn!("Could not save default config: {}", e);
            } else {
                info!("Created default config at {}", path);
            }

            config
        };

        Ok(Self {
            config: Arc::new(RwLock::new(config)),
            path,
        })
    }

    /// Get the current configuration (read-only).
    pub fn get(&self) -> Config {
        // Use try_read to avoid blocking; fall back to default on contention
        self.config
            .try_read()
            .map(|guard| guard.clone())
            .unwrap_or_default()
    }

    /// Get a reference to the config Arc for async access.
    pub fn config_arc(&self) -> Arc<RwLock<Config>> {
        self.config.clone()
    }

    /// Update and persist configuration.
    pub async fn update<F>(&self, updater: F) -> Result<()>
    where
        F: FnOnce(&mut Config),
    {
        let mut config = self.config.write().await;
        updater(&mut config);
        Self::save_config(&self.path, &config)?;
        info!("Configuration updated and saved");
        Ok(())
    }

    /// Save configuration to file.
    fn save_config(path: &str, config: &Config) -> Result<()> {
        // Create parent directory if needed
        if let Some(parent) = Path::new(path).parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config directory: {:?}", parent))?;
        }

        let content = toml::to_string_pretty(config)
            .context("Failed to serialize config")?;

        fs::write(path, content)
            .with_context(|| format!("Failed to write config file: {}", path))?;

        Ok(())
    }

    /// Get blocked domains list.
    pub fn blocked_domains(&self) -> Vec<String> {
        self.get().blocking.domains.clone()
    }

    /// Add a domain to the blocklist.
    pub async fn add_domain(&self, domain: String) -> Result<()> {
        self.update(|config| {
            let normalized = normalize_domain(&domain);
            if !config.blocking.domains.contains(&normalized) {
                config.blocking.domains.push(normalized);
            }
        })
        .await
    }

    /// Remove a domain from the blocklist.
    pub async fn remove_domain(&self, domain: &str) -> Result<bool> {
        let normalized = normalize_domain(domain);
        let mut removed = false;

        self.update(|config| {
            if let Some(pos) = config.blocking.domains.iter().position(|d| d == &normalized) {
                config.blocking.domains.remove(pos);
                removed = true;
            }
        })
        .await?;

        Ok(removed)
    }
}

/// Normalize a domain name (lowercase, remove trailing dot).
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
    fn test_normalize_domain() {
        assert_eq!(normalize_domain("Facebook.COM"), "facebook.com");
        assert_eq!(normalize_domain("twitter.com."), "twitter.com");
        assert_eq!(normalize_domain("  Reddit.com  "), "reddit.com");
    }
}
