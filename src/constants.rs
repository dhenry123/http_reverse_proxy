// Backend
pub const POOL_MAX_IDLE_PER_HOST: usize = 250;
pub const POOL_IDLE_TIMEOUT: u64 = 60;

// Config default
pub const DEFAULT_CONFIG_PATH: &str = "/etc/http_reverse_proxy/config.yaml";
pub const DEFAULT_TLS_CERT_PATH: &str = "/etc/http_reverse_proxy/certs";

// Http header
pub const HTTP_HEADER_X_FORWARDED_FOR: &str = "X-Forwarded-For";
pub const HTTP_HEADER_X_REAL_IP: &str = "X-Real-IP";

// antibot
pub const ANTIBOT_COOKIE_NAME: &str = "antibot";

// Routes
//--> antibot
pub const INTERNAL_ROUTE_ANTIBOT: &str = "_internal_server/antibot";
//--> internal errors
pub const INTERNAL_ROUTE_ERROR_NO_BACKEND_SERVER_AVAILABLE: &str =
    "_internal_server/no_backend_server_available";

// Websocket
pub const INTERNAL_ROUTE_MAKE_WEBSOCKET: &str = "_internal_server/websocket";
pub const SECRET_WS_GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

// TLS
pub const DEFAULT_TLS_CERTIFICAT_FILENAME: &str = "localhost.pem";
