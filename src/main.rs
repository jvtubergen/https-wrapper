use hyper::{
    server::conn::http1,
    service::service_fn,
    Body, Request, Response, Client,
};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tokio_rustls::rustls::{self, ServerConfig};
use std::fs::File;
use std::io::BufReader;
use std::sync::Arc;
use rustls_pemfile::{pkcs12, Identity};

async fn proxy(req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    let client = Client::new();
    let uri = format!("http://127.0.0.1:10000{}", req.uri().path());
    *req.uri_mut() = uri.parse().unwrap();
    client.request(req).await
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Load PFX file and passphrase
    let pfx_file = File::open("cert.pfx")?;
    let pfx = pkcs12::pkcs12_from_der(&mut BufReader::new(pfx_file), "your_passphrase")?;

    // Extract identity (cert + key)
    let identity = Identity {
        cert_chain: pfx.cert.chain,
        key: pfx.key,
    };

    // Configure TLS
    let config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(identity.cert_chain, identity.key)?;

    let addr = "127.0.0.1:8443".parse::<SocketAddr>()?;
    let listener = TcpListener::bind(addr).await?;
    let tls_acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(config));

    println!("HTTPS reverse proxy running on https://{}", addr);
    println!("Proxying to HTTP server at http://127.0.0.1:10000");

    loop {
        let (stream, _) = listener.accept().await?;
        let tls_acceptor = tls_acceptor.clone();
        tokio::spawn(async move {
            let stream = tls_acceptor.accept(stream).await.unwrap();
            let service = service_fn(proxy);
            if let Err(err) = http1::Builder::new()
                .serve_connection(stream, service)
                .await
            {
                eprintln!("Error serving connection: {}", err);
            }
        });
    }
}
