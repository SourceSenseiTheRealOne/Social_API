use std::collections::HashMap;
use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub http_port: u16,

    pub database_url: String,
    pub read_database_url: String,
    pub db_max_connections: u32,
    pub db_read_max_connections: u32,

    pub redis_url: String,

    pub content_api_urls: HashMap<String, String>,

    pub profile_api_url: String,

    pub rate_limit_write: u32,
    pub rate_limit_read: u32, 

    pub cache_ttl_count: u64,
    pub cache_ttl_content_validation: u64,

    pub cb_failure_threshold: u32,
    pub cb_failure_rate_threshold: u32,
    pub cb_open_duration_secs: u64,
    pub cb_half_open_successes: u32,
}

impl Config {
    pub fn from_env() -> Self {
        let config = Config {
            http_port: parse_env_or("HTTP_PORT", 8080),

            database_url: require_env("DATABASE_URL"),
            read_database_url: require_env("READ_DATABASE_URL"),
            db_max_connections: parse_env_or("DB_MAX_CONNECTIONS", 10),
            db_read_max_connections: parse_env_or("DB_READ_MAX_CONNECTIONS", 20),

            redis_url: require_env("REDIS_URL"),

            content_api_urls: discover_content_api_urls(),

            profile_api_url: require_env("PROFILE_API_URL"),

            rate_limit_write: parse_env_or("RATE_LIMIT_WRITE", 30),
            rate_limit_read: parse_env_or("RATE_LIMIT_READ", 1000),

            cache_ttl_count: parse_env_or("CACHE_TTL_COUNT", 60),
            cache_ttl_content_validation: parse_env_or("CACHE_TTL_CONTENT_VALIDATION", 300),

            cb_failure_threshold: parse_env_or("CB_FAILURE_THRESHOLD", 5),
            cb_failure_rate_threshold: parse_env_or("CB_FAILURE_RATE_THRESHOLD", 50),
            cb_open_duration_secs: parse_env_or("CB_OPEN_DURATION_SECS", 30),
            cb_half_open_successes: parse_env_or("CB_HALF_OPEN_SUCCESSES", 3),
        };

        if config.content_api_urls.is_empty() {
            panic!(
                "No content API URLs configured. Set CONTENT_API_{{TYPE}}_URL env vars \
                 (e.g., CONTENT_API_POST_URL=http://localhost:8081)"
            );
        }

        tracing::info!(
            content_types = ?config.content_api_urls.keys().collect::<Vec<_>>(),
            "Configuration loaded"
        );

        config
    }

    pub fn valid_content_types(&self) -> Vec<&str> {
        self.content_api_urls.keys().map(|s| s.as_str()).collect()
    }

    pub fn content_api_url(&self, content_type: &str) -> Option<&str> {
        self.content_api_urls.get(content_type).map(|s| s.as_str())
    }
}

fn discover_content_api_urls() -> HashMap<String, String> {
    let prefix = "CONTENT_API_";
    let suffix = "_URL";

    env::vars()
        .filter(|(key, _)| key.starts_with(prefix) && key.ends_with(suffix))
        .map(|(key, value)| {
            let content_type = key
                .strip_prefix(prefix)
                .unwrap()
                .strip_suffix(suffix)
                .unwrap()
                .to_lowercase();
            (content_type, value)
        })
        .collect()
}

fn require_env(key: &str) -> String {
    env::var(key).unwrap_or_else(|_| panic!("Required environment variable {key} is not set"))
}

fn parse_env_or<T: std::str::FromStr>(key: &str, default: T) -> T
where
    T::Err: std::fmt::Display,
{
    match env::var(key) {
        Ok(val) => val
            .parse()
            .unwrap_or_else(|e| panic!("Invalid value for {key}: {e}")),
        Err(_) => default,
    }
}
