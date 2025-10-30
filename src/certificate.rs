use openssl::pkcs12::Pkcs12;
use rustls_pki_types::{CertificateDer, PrivateKeyDer};
use std::ffi::OsStr;
use std::fs;
use std::io::BufReader;
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
    validate_extension: bool,
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

    // Validate file extension for PFX (only when using positional arguments)
    if validate_extension {
        if let Some(ext) = path.extension().and_then(OsStr::to_str) {
            let ext = ext.to_lowercase();
            if ext != "pfx" && ext != "p12" {
                return Err(format!("Invalid PFX file extension '{}'. Expected .pfx or .p12", ext).into());
            }
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

/// Load certificate and key from separate PEM files
pub fn load_pem_certificate(
    cert_path: &str,
    key_path: &str,
) -> Result<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>), Box<dyn std::error::Error + Send + Sync>> {
    let cert_file_path = Path::new(cert_path);
    let key_file_path = Path::new(key_path);

    // Check if files exist
    if !cert_file_path.exists() {
        return Err(format!("Certificate file not found: {}", cert_path).into());
    }
    if !key_file_path.exists() {
        return Err(format!("Key file not found: {}", key_path).into());
    }

    // Load certificate chain
    let cert_file = fs::File::open(cert_file_path)
        .map_err(|e| format!("Failed to open certificate file {}: {}", cert_path, e))?;
    let mut cert_reader = BufReader::new(cert_file);

    let certs = rustls_pemfile::certs(&mut cert_reader)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to parse PEM certificate: {}", e))?;

    if certs.is_empty() {
        return Err("No certificates found in PEM file".into());
    }

    println!("Loaded {} certificate(s) from PEM file ({} bytes total)",
             certs.len(),
             certs.iter().map(|c| c.len()).sum::<usize>());

    // Load private key
    let key_file = fs::File::open(key_file_path)
        .map_err(|e| format!("Failed to open key file {}: {}", key_path, e))?;
    let mut key_reader = BufReader::new(key_file);

    let private_key = rustls_pemfile::private_key(&mut key_reader)
        .map_err(|e| format!("Failed to parse PEM private key: {}", e))?
        .ok_or("No private key found in PEM file")?;

    println!("Loaded private key from PEM file");

    Ok((certs, private_key))
}

/// Detect certificate type by file extension
pub fn detect_cert_type(path: &str) -> Result<CertType, String> {
    let path = Path::new(path);

    if let Some(ext) = path.extension().and_then(OsStr::to_str) {
        let ext = ext.to_lowercase();
        match ext.as_str() {
            "pfx" | "p12" => Ok(CertType::Pfx),
            "pem" | "crt" | "cer" | "cert" | "key" => Ok(CertType::Pem),
            _ => Err(format!("Unsupported certificate file extension: .{}", ext)),
        }
    } else {
        Err("Certificate file has no extension".into())
    }
}

#[derive(Debug, PartialEq)]
pub enum CertType {
    Pfx,
    Pem,
}
