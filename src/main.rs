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

    // Load certificate and private key using the certificate module
    let (certs, private_key) = certificate::load_certificate(
        &args.certificate,
        args.password.as_deref(),
    )?;

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
