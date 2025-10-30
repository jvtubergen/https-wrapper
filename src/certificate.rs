use openssl::pkcs12::Pkcs12;
use rustls_pki_types::{CertificateDer, PrivateKeyDer};
use std::ffi::OsStr;
use std::fs;
use std::path::Path;

/// Parse a PFX file from bytes - adapted from forge
fn parse_pfx_bytes(data: &[u8], password: &str) -> Result<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>), Box<dyn std::error::Error + Send + Sync>> {
    // Validate input data
    if data.is_empty() {
        return Err("Empty PFX data provided".into());
    }

    // Parse the PKCS12 structure
    let pkcs12 = Pkcs12::from_der(data)
        .map_err(|e| format!("Failed to parse PFX structure: {}", e))?;

    // Extract the contents with the provided password
    let parsed = pkcs12.parse2(password).map_err(|e| {
        if password.is_empty() {
            format!(
                "Failed to parse PFX file: {}. This file may require a password. Use --password option.",
                e
            )
        } else {
            format!("Failed to parse PFX file with provided password: {}", e)
        }
    })?;

    // Convert OpenSSL types to rustls types
    let mut certs: Vec<CertificateDer<'static>> = Vec::new();

    // Add the main certificate
    if let Some(cert) = parsed.cert {
        let cert_der = cert.to_der()
            .map_err(|e| format!("Failed to encode certificate to DER: {}", e))?;
        println!("Found main certificate ({} bytes)", cert_der.len());
        certs.push(CertificateDer::from(cert_der));
    }

    // Add any chain certificates
    if let Some(chain) = parsed.ca {
        for cert in chain {
            let cert_der = cert.to_der()
                .map_err(|e| format!("Failed to encode chain certificate to DER: {}", e))?;
            println!("Found chain certificate ({} bytes)", cert_der.len());
            certs.push(CertificateDer::from(cert_der));
        }
    }

    // Extract private key
    let private_key = parsed.pkey
        .ok_or("No private key found in PFX file")?;

    // Convert to PKCS#8 format for rustls
    let key_der = private_key.private_key_to_pkcs8()
        .map_err(|e| format!("Failed to encode private key to PKCS#8: {}", e))?;
    println!("Extracted private key ({} bytes)", key_der.len());

    Ok((certs, PrivateKeyDer::Pkcs8(key_der.into())))
}

/// Load certificate from PFX file - adapted from forge parser
pub fn load_certificate(
    certificate_path: &str,
    password: Option<&str>,
) -> Result<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>), Box<dyn std::error::Error + Send + Sync>> {
    let path = Path::new(certificate_path);

    // Check if file exists
    if !path.exists() {
        return Err(format!("File not found: {}", path.display()).into());
    }

    // Verify that the file is readable
    if let Err(e) = fs::metadata(path) {
        return Err(format!(
            "Cannot read file {}: {}",
            path.display(),
            e
        ).into());
    }

    // Validate file extension (if it has one)
    if let Some(ext) = path.extension().and_then(OsStr::to_str) {
        let ext = ext.to_lowercase();
        if ext != "pfx" && ext != "p12" {
            return Err(format!("Invalid file extension: {}", ext).into());
        }
    }

    // Read the file
    let pfx_data = fs::read(path)
        .map_err(|e| format!("Failed to read certificate file {}: {}", path.display(), e))?;

    // Validate file size
    if pfx_data.is_empty() {
        return Err("File is empty".into());
    }

    println!("Read {} bytes from certificate file", pfx_data.len());

    // Extract certificates and private key with password
    let password = password.unwrap_or("");
    println!("Attempting to decrypt PFX with {}password", if password.is_empty() { "empty " } else { "" });

    parse_pfx_bytes(&pfx_data, password)
}
