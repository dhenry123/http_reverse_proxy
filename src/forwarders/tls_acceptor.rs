use rustls::server::{ClientHello, ResolvesServerCert};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio_rustls::TlsAcceptor;

use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::{ServerConfig, crypto::aws_lc_rs::sign::any_supported_type, sign::CertifiedKey};

use crate::structs::{GenericError, GenericResult};

// Custom certificate resolver that can access the SNI
#[derive(Debug)]
struct SnAwareCertResolver {
    // Your existing certificate resolver or certificate store
    inner: Arc<dyn ResolvesServerCert>,
    fallback_cert: Option<Arc<CertifiedKey>>,
}

/**
 * @todo refactor currently used for debug
 */
impl ResolvesServerCert for SnAwareCertResolver {
    fn resolve(
        &self,
        client_hello: ClientHello,
    ) -> Option<Arc<tokio_rustls::rustls::sign::CertifiedKey>> {
        // Try the main resolver first
        if let Some(cert) = self.inner.resolve(client_hello) {
            println!("Return legitimate cert");
            return Some(cert);
        }
        // If no match, use fallback if available
        if let Some(fallback) = &self.fallback_cert {
            println!("Return fallback cert");
            return Some(fallback.clone());
        }
        None
    }
}

// Creates a TLS configuration from loaded certificates
pub fn create_tls_config(
    cert_map: HashMap<String, (Vec<CertificateDer<'static>>, PrivateKeyDer<'_>)>,
) -> GenericResult<Arc<ServerConfig>> {
    let mut cert_resolver = rustls::server::ResolvesServerCertUsingSni::new();

    let mut fallback: Option<Arc<CertifiedKey>> = None;

    for (domain, (cert_chain, private_key)) in cert_map {
        let key = any_supported_type(&private_key)
            .map_err(|e| format!("Unsupported private key: {}", e))?;
        let cert_key = CertifiedKey::new(cert_chain, key);
        if domain == "localhost".to_string() {
            fallback = Some(Arc::new(cert_key));
            continue;
        }
        cert_resolver
            .add(&domain, cert_key)
            .map_err(|e| format!("Failed to add certificate for {}: {}", domain, e))?;
        println!("Tls domain loaded: {}", domain);
    }

    // Wrap your resolver with our SNI-aware version
    let sni_aware_resolver = Arc::new(SnAwareCertResolver {
        inner: Arc::new(cert_resolver),
        fallback_cert: fallback,
    });

    // Build final configuration
    let config = ServerConfig::builder()
        .with_no_client_auth()
        .with_cert_resolver(sni_aware_resolver);

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
                .ok_or_else(|| GenericError::from("Invalid PEM filename"))?
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

pub fn tls_acceptor_init(certs_path: PathBuf) -> Result<TlsAcceptor, GenericError> {
    // Load all certificates from directory
    let cert_map = load_combined_pems(certs_path.clone())?;
    let tls_config = create_tls_config(cert_map)?;
    Ok(TlsAcceptor::from(tls_config))
}
