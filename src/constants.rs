// Backend
pub const POOL_MAX_IDLE_PER_HOST: usize = 250;
pub const POOL_IDLE_TIMEOUT: u64 = 60;

// Config default
pub const DEFAULT_CONFIG_PATH: &str = "/etc/http_reverse_proxy/config.yaml";
pub const DEFAULT_TLS_CERT_PATH: &str = "/etc/http_reverse_proxy/certs";

// Http header
pub const HTTP_HEADER_X_FORWARDED_FOR: &str = "X-Forwarded-For";
pub const HTTP_HEADER_X_REAL_IP: &str = "X-Real-IP";

//internal ports
pub const HTTP_INTERNAL_ERROR_PORT: u16 = 2201;
pub const HTTP_INTERNAL_ANTIBOT_PORT: u16 = 2202;

// antibot
pub const ANTIBOT_COOKIE_NAME: &str = "antibot";
