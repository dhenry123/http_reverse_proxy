use rcgen::{CertificateParams, DistinguishedName, DnType, KeyPair, SanType, date_time_ymd};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use crate::constants::DEFAULT_TLS_CERTIFICAT_FILENAME;
use crate::structs::GenericError;

pub fn generate_default_cert(path: &PathBuf) -> Result<(), GenericError> {
    let mut params: CertificateParams = Default::default();
    params.not_before = date_time_ymd(1975, 1, 1);
    params.not_after = date_time_ymd(4096, 1, 1);
    params.distinguished_name = DistinguishedName::new();
    params
        .distinguished_name
        .push(DnType::OrganizationName, "localhost");
    params
        .distinguished_name
        .push(DnType::CommonName, "localhost");
    params.subject_alt_names = vec![SanType::DnsName("localhost".try_into()?)];

    let key_pair = KeyPair::generate()?;
    let cert = params.self_signed(&key_pair)?;

    match save_combined_pem(
        cert.pem().as_str(),
        key_pair.serialize_pem().as_str(),
        &path,
    ) {
        Ok(_) => Ok(()),
        Err(e) => Err(Box::new(e)),
    }
}

fn save_combined_pem(cert_pem: &str, key_pem: &str, path: &PathBuf) -> std::io::Result<()> {
    let mut file = File::create(path.join(DEFAULT_TLS_CERTIFICAT_FILENAME))?;

    // Write certificate first
    writeln!(file, "{}", cert_pem)?;

    // Then write private key
    writeln!(file, "{}", key_pem)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::NamedTempFile;

    #[test]
    fn test_generate_default_cert_success() {
        // Create a temporary file for testing
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_str().unwrap();

        // Test certificate generation
        let result = generate_default_cert(&PathBuf::from(path));
        assert!(result.is_ok(), "Certificate generation failed");

        // Verify the file was created and has content
        let content = fs::read_to_string(path).unwrap();
        // println!("{}", content);
        assert!(!content.is_empty(), "File should not be empty");

        // Check that both certificate and key are present
        assert!(
            content.contains("-----BEGIN CERTIFICATE-----"),
            "Missing certificate"
        );
        assert!(
            content.contains("-----BEGIN PRIVATE KEY-----"),
            "Missing private key"
        );
    }

    #[test]
    fn test_generate_default_cert_invalid_path() {
        // Test with an invalid path
        let result = generate_default_cert(&PathBuf::from("/invalid/path/cert.pem"));
        assert!(result.is_err(), "Should fail with invalid path");
    }

    #[test]
    fn test_save_combined_pem_success() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_str().unwrap();

        let cert = "-----BEGIN CERTIFICATE-----\nTEST\n-----END CERTIFICATE-----";
        let key = "-----BEGIN PRIVATE KEY-----\nTEST\n-----END PRIVATE KEY-----";

        let result = save_combined_pem(cert, key, &PathBuf::from(path));
        assert!(result.is_ok(), "Saving PEM failed");

        let content = fs::read_to_string(path).unwrap();
        assert!(content.contains(cert), "Certificate not saved correctly");
        assert!(content.contains(key), "Private key not saved correctly");
    }

    #[test]
    fn test_save_combined_pem_invalid_path() {
        let cert = "-----BEGIN CERTIFICATE-----\nTEST\n-----END CERTIFICATE-----";
        let key = "-----BEGIN PRIVATE KEY-----\nTEST\n-----END PRIVATE KEY-----";

        let result = save_combined_pem(cert, key, &PathBuf::from("/invalid/path/cert.pem"));
        assert!(result.is_err(), "Should fail with invalid path");
    }
}
