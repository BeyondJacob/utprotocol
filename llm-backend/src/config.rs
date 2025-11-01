use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub cache: CacheConfig,

    #[serde(default)]
    pub providers: ProvidersConfig,

    #[serde(default)]
    pub server: ServerConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Maximum number of entries in the cache
    #[serde(default = "default_max_entries")]
    pub max_entries: usize,

    /// Time-to-live for cache entries in seconds
    #[serde(default = "default_ttl_seconds")]
    pub ttl_seconds: u64,

    /// Cache eviction policy (lru or random)
    #[serde(default = "default_eviction_policy")]
    pub eviction_policy: String,

    /// Semantic similarity threshold for cache hits (0.0 to 1.0)
    #[serde(default = "default_similarity_threshold")]
    pub similarity_threshold: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvidersConfig {
    #[serde(default)]
    pub groq: ProviderConfig,

    #[serde(default)]
    pub ollama: ProviderConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// Request timeout in seconds
    #[serde(default = "default_timeout_seconds")]
    pub timeout_seconds: u64,

    /// Maximum number of retries for failed requests
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,

    /// Rate limit: requests per minute
    #[serde(default = "default_rate_limit_per_minute")]
    pub rate_limit_per_minute: u32,

    /// Circuit breaker: failures before opening circuit
    #[serde(default = "default_circuit_breaker_threshold")]
    pub circuit_breaker_threshold: u32,

    /// Circuit breaker: cooldown period in seconds
    #[serde(default = "default_circuit_breaker_cooldown")]
    pub circuit_breaker_cooldown_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Server host
    #[serde(default = "default_host")]
    pub host: String,

    /// Server port
    #[serde(default = "default_port")]
    pub port: u16,
}

// Default value functions
fn default_max_entries() -> usize { 10000 }
fn default_ttl_seconds() -> u64 { 3600 } // 1 hour
fn default_eviction_policy() -> String { "lru".to_string() }
fn default_similarity_threshold() -> f32 { 0.92 } // Semantic cache threshold
fn default_timeout_seconds() -> u64 { 30 }
fn default_max_retries() -> u32 { 3 }
fn default_rate_limit_per_minute() -> u32 { 60 }
fn default_circuit_breaker_threshold() -> u32 { 5 }
fn default_circuit_breaker_cooldown() -> u64 { 60 }
fn default_host() -> String { "127.0.0.1".to_string() }
fn default_port() -> u16 { 3001 }

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_entries: default_max_entries(),
            ttl_seconds: default_ttl_seconds(),
            eviction_policy: default_eviction_policy(),
            similarity_threshold: default_similarity_threshold(),
        }
    }
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            timeout_seconds: default_timeout_seconds(),
            max_retries: default_max_retries(),
            rate_limit_per_minute: default_rate_limit_per_minute(),
            circuit_breaker_threshold: default_circuit_breaker_threshold(),
            circuit_breaker_cooldown_seconds: default_circuit_breaker_cooldown(),
        }
    }
}

impl Default for ProvidersConfig {
    fn default() -> Self {
        Self {
            groq: ProviderConfig::default(),
            ollama: ProviderConfig::default(),
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            cache: CacheConfig::default(),
            providers: ProvidersConfig::default(),
            server: ServerConfig::default(),
        }
    }
}

impl Config {
    /// Load configuration from a TOML file
    pub fn from_file(path: &str) -> anyhow::Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&contents)?;
        Ok(config)
    }

    /// Load configuration from file or use defaults
    pub fn load() -> Self {
        Self::from_file("config.toml")
            .unwrap_or_else(|_| {
                tracing::warn!("Failed to load config.toml, using defaults");
                Self::default()
            })
    }

    /// Get cache TTL as Duration
    pub fn cache_ttl(&self) -> Duration {
        Duration::from_secs(self.cache.ttl_seconds)
    }

    /// Get provider timeout as Duration
    pub fn provider_timeout(&self, provider: &str) -> Duration {
        let config = match provider {
            "groq" => &self.providers.groq,
            "ollama" => &self.providers.ollama,
            _ => &self.providers.groq,
        };
        Duration::from_secs(config.timeout_seconds)
    }
}
