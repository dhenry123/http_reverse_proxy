use super::forwarder_helper::{create_tls_config, load_combined_pems};
use crate::structs::GenericError;
use std::path::PathBuf;
use tokio_rustls::TlsAcceptor;

pub fn init_tls_acceptor(certs_path: PathBuf) -> Result<TlsAcceptor, GenericError> {
    // Load all certificates from directory
    let cert_map = load_combined_pems(certs_path.clone())?;
    let tls_config = create_tls_config(cert_map)?;
    Ok(TlsAcceptor::from(tls_config))
}
