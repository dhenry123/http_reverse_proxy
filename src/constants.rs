// Backend
pub const POOL_MAX_IDLE_PER_HOST: usize = 50;
pub const POOL_IDLE_TIMEOUT: u64 = 60;

// Config default
pub const DEFAULT_CONFIG_PATH: &str = "/etc/http_reverse_proxy/config.yaml";
pub const DEFAULT_TLS_CERT_PATH: &str = "/etc/http_reverse_proxy/certs";

// Http header
pub const HTTP_HEADER_X_FORWARDED_FOR: &str = "X-Forwarded-For";
pub const HTTP_HEADER_X_REAL_IP: &str = "X-Real-IP";
pub const HTTP_HEADER_HOST: &str = "Host";
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

// API Rest
pub const API_HEADER_VALUE_ACCESS_CONTROL_ALLOW_ORIGIN: &str = "*";
pub const API_VERSION: &str = "api/v1";
pub const API_LISTENING_ADDR: &str = "127.0.0.1";
pub const API_LISTENING_PORT: u16 = 27001;
pub const API_FRONTENDS_LIST: &str = "list/frontends";
pub const API_BACKENDS_LIST: &str = "list/backends";
pub const API_SERVERS_LIST: &str = "list/servers";

pub const API_SERVERS_ACTIVE: &str = "server/active";

pub const API_METRICS_GET_HITS: &str = "metrics/hits";

// Json response
pub const JSON_STATUS_LABEL_SUCCESS: &str = "success";
pub const JSON_STATUS_LABEL_ERROR: &str = "error";

pub const RUNTIMEBACKENDS_INTERVAL_CHECK: u64 = 5;
