use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::rustls::ServerConfig;
use std::net::SocketAddr;
use std::sync::Arc;
use clap::Parser;

mod certificate;

#[derive(Parser, Debug)]
#[command(name = "https-wrapper")]
#[command(about = "Minimalistic HTTPS wrapper to provide TLS layer to your HTTP server", long_about = None)]
struct Args {
    /// Input address (HTTPS server listens on this address, format: ip:port)
    #[arg(value_name = "INPUT_ADDRESS")]
    input_address: String,

    /// Output address (HTTP server to forward requests to, format: ip:port)
    #[arg(value_name = "OUTPUT_ADDRESS")]
    output_address: String,

    /// Path to certificate file (positional argument, .pfx/.p12 or .pem/.crt)
    #[arg(value_name = "CERTIFICATE", conflicts_with_all = ["pfx", "cert"])]
    certificate: Option<String>,

    /// Password for PFX or second positional arg as key file for PEM
    #[arg(value_name = "PASSWORD_OR_KEY")]
    password_or_key: Option<String>,

    // Named arguments
    /// Path to PFX certificate file (.pfx or .p12)
    #[arg(long, value_name = "PFX_FILE", conflicts_with_all = ["cert", "key"])]
    pfx: Option<String>,

    /// Path to PEM certificate file (.pem or .crt)
    #[arg(long, value_name = "CERT_FILE", requires = "key", conflicts_with = "pfx")]
    cert: Option<String>,

    /// Path to private key file (.pem or .key)
    #[arg(long, value_name = "KEY_FILE", requires = "cert")]
    key: Option<String>,

    /// Password for PFX file
    #[arg(long, value_name = "PASSWORD")]
    password: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Parse CLI arguments
    let args = Args::parse();

    // Load certificate and private key based on provided arguments
    let (certs, private_key) = if let Some(pfx_path) = &args.pfx {
        // Named mode: --pfx [--password] (no extension validation)
        certificate::load_certificate(pfx_path, args.password.as_deref(), false)?
    } else if let Some(cert_path) = &args.cert {
        // Named mode: --cert --key (no extension validation)
        let key_path = args.key.as_ref().unwrap(); // Safe due to clap's requires constraint
        certificate::load_pem_certificate(cert_path, key_path)?
    } else if let Some(cert_path) = &args.certificate {
        // Positional mode: detect format by extension
        let cert_type = certificate::detect_cert_type(cert_path)
            .map_err(|e| format!("Failed to detect certificate type: {}", e))?;

        match cert_type {
            certificate::CertType::Pfx => {
                // PFX format: certificate [password] (with extension validation)
                certificate::load_certificate(cert_path, args.password_or_key.as_deref(), true)?
            }
            certificate::CertType::Pem => {
                // PEM format: certificate keyfile (no extension validation needed)
                let key_path = args.password_or_key.as_ref()
                    .ok_or("PEM certificate requires a key file as the second argument")?;
                certificate::load_pem_certificate(cert_path, key_path)?
            }
        }
    } else {
        return Err("No certificate specified. Use either positional arguments or named flags (--pfx or --cert/--key)".into());
    };

    // Configure TLS
    let config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, private_key)?;

    let addr = args.input_address.parse::<SocketAddr>()?;
    let listener = TcpListener::bind(addr).await?;
    let tls_acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(config));

    println!("HTTPS reverse proxy running on https://{}", addr);
    println!("Proxying to HTTP server at http://{}", args.output_address);

    let output_address = args.output_address.clone();
    loop {
        let (client_stream, _) = listener.accept().await?;
        let tls_acceptor = tls_acceptor.clone();
        let output_address = output_address.clone();

        tokio::spawn(async move {
            // TLS handshake
            let mut tls_stream = match tls_acceptor.accept(client_stream).await {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("TLS handshake error: {}", e);
                    return;
                }
            };

            // Connect to backend HTTP server
            let mut backend_stream = match TcpStream::connect(&output_address).await {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Backend connection error: {}", e);
                    return;
                }
            };

            println!("Forwarding request to http://{}", output_address);

            // Bidirectional TCP forwarding (TLS <-> HTTP)
            if let Err(e) = tokio::io::copy_bidirectional(
                &mut tls_stream,
                &mut backend_stream
            ).await {
                eprintln!("Proxy forwarding error: {}", e);
            }
        });
    }
}
