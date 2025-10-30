use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::rustls::ServerConfig;
use rustls_pki_types::{CertificateDer, PrivateKeyDer};
use std::fs;
use std::net::SocketAddr;
use std::sync::Arc;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "https-wrapper")]
#[command(about = "Minimalistic HTTPS wrapper to provide TLS layer to your HTTP server", long_about = None)]
struct Args {
    /// Input port (HTTPS server listens on this port)
    #[arg(value_name = "INPUT_PORT")]
    input_port: u16,

    /// Output port (HTTP server to forward requests to)
    #[arg(value_name = "OUTPUT_PORT")]
    output_port: u16,

    /// Path to certificate file (.pfx or .chain)
    #[arg(value_name = "CERTIFICATE")]
    certificate: String,

    /// Password for the certificate (optional, if PFX is not encrypted)
    #[arg(value_name = "PASSWORD")]
    password: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Parse CLI arguments
    let args = Args::parse();

    // Load PFX file and parse it
    let pfx_data = fs::read(&args.certificate)?;
    let pfx = p12::PFX::parse(&pfx_data)?;

    // Extract certificates and private key from PFX
    let password = args.password.as_deref().unwrap_or("");
    let bags = pfx.bags(password)?;

    let mut certs: Vec<CertificateDer<'static>> = Vec::new();
    let mut private_key: Option<PrivateKeyDer<'static>> = None;

    for bag in bags {
        match &bag.bag {
            p12::SafeBagKind::CertBag(cert_bag) => {
                if let p12::CertBag::X509(cert_data) = cert_bag {
                    certs.push(CertificateDer::from(cert_data.clone()));
                }
            }
            p12::SafeBagKind::Pkcs8ShroudedKeyBag(_) => {
                // Decrypt the key using the password
                if let Some(key_bytes) = bag.bag.get_key(password.as_bytes()) {
                    private_key = Some(PrivateKeyDer::Pkcs8(key_bytes.into()));
                }
            }
            _ => {}
        }
    }

    let private_key = private_key.ok_or("No private key found in PFX file")?;

    // Configure TLS
    let config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, private_key)?;

    let addr = format!("127.0.0.1:{}", args.input_port).parse::<SocketAddr>()?;
    let listener = TcpListener::bind(addr).await?;
    let tls_acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(config));

    println!("HTTPS reverse proxy running on https://{}", addr);
    println!("Proxying to HTTP server at http://127.0.0.1:{}", args.output_port);

    let output_port = args.output_port;
    loop {
        let (client_stream, _) = listener.accept().await?;
        let tls_acceptor = tls_acceptor.clone();

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
            let mut backend_stream = match TcpStream::connect(
                format!("127.0.0.1:{}", output_port)
            ).await {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Backend connection error: {}", e);
                    return;
                }
            };

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
