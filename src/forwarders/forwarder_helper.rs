use std::{collections::HashMap, error::Error, fs, path::PathBuf, sync::Arc, time::Duration};

use arc_swap::{ArcSwap, ArcSwapAny};
use hyper::{Request, body, header::HeaderValue};
use hyper_tls::HttpsConnector;
use hyper_util::{
    client::legacy::{Client, connect::HttpConnector},
    rt::TokioExecutor,
};
use rustls::{ServerConfig, crypto::aws_lc_rs::sign::any_supported_type, sign::CertifiedKey};

use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use uuid::Uuid;

use crate::{
    constants::{ANTIBOT_COOKIE_NAME, POOL_IDLE_TIMEOUT, POOL_MAX_IDLE_PER_HOST},
    structs::{BackendServer, GenericError, GenericResult, ProxyConfig},
};

use super::servers_tracker::ServerTracker;
use cookie::Cookie;

pub fn build_upstream_uri(backend_server: BackendServer, is_web_socket: bool) -> String {
    let mut upstream: String;
    // protocol
    let proto = if is_web_socket {
        "ws"
    } else {
        backend_server.protocol.as_ref()
    };
    // scheme
    if backend_server.tls {
        upstream = format!("{}s://{}", proto, backend_server.host);
    } else {
        upstream = format!("{}://{}", proto, backend_server.host);
    }
    //port
    upstream = format!("{}:{}", upstream, backend_server.port);
    // Optional path
    if backend_server.path.is_some() {
        upstream = format!("{}:{}", upstream, backend_server.path.clone().unwrap());
    }
    return upstream;
}

// Creates a TLS configuration from loaded certificates
pub fn create_tls_config(
    cert_map: HashMap<String, (Vec<CertificateDer<'static>>, PrivateKeyDer<'_>)>,
) -> GenericResult<Arc<ServerConfig>> {
    let mut cert_resolver = rustls::server::ResolvesServerCertUsingSni::new();

    for (domain, (cert_chain, private_key)) in cert_map {
        let key = any_supported_type(&private_key)
            .map_err(|e| format!("Unsupported private key: {}", e))?;
        let cert_key = CertifiedKey::new(cert_chain, key);
        cert_resolver
            .add(&domain, cert_key)
            .map_err(|e| format!("Failed to add certificate for {}: {}", domain, e))?;
        println!("Tls domain loaded: {}", domain);
    }

    // Support both TLS 1.2 and 1.3 for better compatibility
    //let versions: &[&SupportedProtocolVersion] = &[&TLS12, &TLS13];

    // Build final configuration
    let config = ServerConfig::builder()
        .with_no_client_auth()
        .with_cert_resolver(Arc::new(cert_resolver));
    //.with_safe_default_cipher_suites()
    //.with_safe_default_kx_groups()
    //.with_protocol_versions(versions)
    // .map_err(|e| format!("TLS version configuration failed: {}", e))?
    // .with_no_client_auth()
    // .with_cert_resolver(Arc::new(cert_resolver)); // Proper builder method

    Ok(Arc::new(config))
}

// Load certificates from combined PEM files (cert + key in one file)
pub fn load_combined_pems(
    cert_dir: PathBuf,
) -> Result<
    HashMap<std::string::String, (Vec<CertificateDer<'static>>, PrivateKeyDer<'static>)>,
    GenericError,
> {
    let mut cert_map = HashMap::new();

    println!("Configuration certs path: {:?}", cert_dir);
    let certs_files_list = fs::read_dir(cert_dir).map_err(|e| -> GenericError { Box::new(e) })?;
    for entry in certs_files_list {
        let entry = entry.map_err(|e| -> GenericError { Box::new(e) })?;
        let path = entry.path();
        if path.extension().map_or(false, |ext| ext == "pem") {
            let domain = path
                .file_stem()
                .and_then(|s| s.to_str())
                .ok_or_else(|| {
                    Box::<dyn Error + Send + Sync + 'static>::from("Invalid PEM filename")
                })?
                .to_string();

            let file_contents = fs::read(&path).map_err(|e| -> GenericError { Box::new(e) })?;
            let mut reader = std::io::Cursor::new(file_contents);

            // Read all items from the PEM file, collecting any errors
            let items: Vec<rustls_pemfile::Item> = rustls_pemfile::read_all(&mut reader)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| -> GenericError { Box::new(e) })?;

            // Process items into certificates and private key
            let mut cert_chain = Vec::new();
            let mut private_key = None;

            for item in items {
                match item {
                    rustls_pemfile::Item::X509Certificate(cert) => {
                        cert_chain.push(rustls::pki_types::CertificateDer::from(cert.to_vec()));
                    }
                    rustls_pemfile::Item::Pkcs8Key(key) if private_key.is_none() => {
                        private_key = Some(rustls::pki_types::PrivateKeyDer::from(key));
                    }
                    _ => {}
                }
            }

            if cert_chain.is_empty() {
                eprintln!("Warning: No certificates found in {}", path.display());
                continue;
            }

            let private_key = match private_key {
                Some(key) => key,
                None => {
                    eprintln!("Warning: No private key found in {}", path.display());
                    continue;
                }
            };

            cert_map.insert(domain, (cert_chain, private_key));
        }
    }
    Ok(cert_map)
}

/**
 * return an http connector
 */
pub fn get_http_client() -> Client<hyper_tls::HttpsConnector<HttpConnector>, body::Incoming> {
    let mut http_connector = HttpConnector::new();
    http_connector.set_nodelay(true);
    http_connector.set_keepalive(Some(std::time::Duration::from_secs(60)));

    let https_connector = HttpsConnector::new();

    Client::builder(TokioExecutor::new())
        .pool_max_idle_per_host(POOL_MAX_IDLE_PER_HOST)
        .pool_idle_timeout(Duration::from_secs(POOL_IDLE_TIMEOUT))
        .http1_preserve_header_case(true)
        .http2_keep_alive_interval(Duration::from_secs(30))
        .build::<_, body::Incoming>(https_connector)
}

/**
 * return the final uri selecting the backend with roundrobin
 */
pub fn get_upstream_uri(
    original_host: String,
    servers_tracker: Arc<ArcSwapAny<Arc<ServerTracker>>>,
    is_web_socket: bool,
) -> String {
    // Which backend ?
    let backend_server = servers_tracker
        .load()
        .as_ref()
        .get_next_backend(&original_host);
    //println!("backend_server: {:?}", backend_server);
    if backend_server.is_some() {
        build_upstream_uri(backend_server.unwrap(), is_web_socket)
    } else {
        "".to_string()
    }
}

/**
 * Browser config to look for frontend/host is set with antibot
 */
pub fn is_domain_configured_for_antibot(
    frontend_name: String,
    original_host: String,
    config: Arc<ArcSwap<ProxyConfig>>,
) -> bool {
    let config = config.clone().load();
    let lookup_table = config
        // filter frontend on frontend_name
        .frontends
        .iter()
        .find(|f| f.name == frontend_name)
        // Get acls
        .into_iter()
        .flat_map(|frontend| &frontend.acls)
        .find(|a| a.host == original_host)
        .into_iter()
        .collect::<Vec<_>>();

    if lookup_table.len() == 1 {
        let antibot_state = lookup_table.get(0).unwrap().antibot;
        if antibot_state.is_some() && antibot_state.unwrap() {
            //println!("Configured with antibot");
            return true;
        }
    }
    // println!("Not configured with antibot");
    return false;
}

/**
 * Simple cookie...
 */
pub fn get_cookie_antibot(host: String) -> Cookie<'static> {
    let cookie = Cookie::build((ANTIBOT_COOKIE_NAME, Uuid::new_v4().to_string()))
        .domain(host)
        .path("/")
        .secure(false)
        .http_only(true)
        .same_site(cookie::SameSite::Strict)
        .max_age(cookie::time::Duration::hours(2))
        .build();
    return cookie;
}

pub fn is_cookie_antibot(cookie_http_header: Option<&HeaderValue>) -> bool {
    if cookie_http_header.is_none() {
        return false;
    } else {
        // check antibot cookie
        for cookie in Cookie::split_parse(cookie_http_header.unwrap().to_str().unwrap()) {
            let cookie = cookie;
            if cookie.is_ok() && cookie.unwrap().name() == ANTIBOT_COOKIE_NAME {
                return true;
            }
        }
        return false;
    }
}

// Helper function to check WebSocket request
pub fn is_websocket_request(req: &Request<hyper::body::Incoming>) -> bool {
    req.headers()
        .get("Upgrade")
        .and_then(|h| h.to_str().ok())
        .map(|h| h.eq_ignore_ascii_case("websocket"))
        .unwrap_or(false)
}
