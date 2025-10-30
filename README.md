# https-wrapper

_Minimalistic HTTPS reverse proxy that adds TLS encryption to any HTTP server._

Minimalistic HTTPS reverse proxy written in Rust. Wraps any HTTP server with TLS encryption using PFX/PKCS12 or PEM certificates. Perfect for adding HTTPS to local development servers or production applications that don't natively support TLS.

In comparison to HTTP proxies, this tool uses bidirectional binary data streaming with minimal overhead. By forwarding raw TCP bytes without parsing or rewriting HTTP requests, it achieves better performance while supporting a broader range of protocols including HTTP/1.1, HTTP/2, WebSockets, and Server-Sent Events transparently.

This code constructs a reverse proxy to provide TLS connections and forward any requests to another port (where you have your HTTP server listening at). And the CLI tool is used by providing the following information:
- What address (ip:port) this (HTTPS reverse proxy) server should listen to.
- What address (ip:port) this server should forward the request information to.
- The TLS certificate (supports both PFX/PKCS12 and PEM formats).

<!-- [https server (ip:port + certificate)] -> [http server (ip:port)] -->

## Quick Start

```bash
# With PFX certificate
https-wrapper 0.0.0.0:443 127.0.0.1:8080 cert.pfx mypassword

# With PEM certificates (like Let's Encrypt)
https-wrapper 0.0.0.0:443 127.0.0.1:8080 fullchain.pem privkey.pem
```

This will start an HTTPS server on port 443 that forwards all requests to your HTTP server running on localhost:8080.

## Installation

You'll need Rust installed to build this. If you don't have Rust yet, get it from [rustup.rs](https://rustup.rs/).

```bash
cargo build --release
```

The binary will be at `target/release/https-wrapper`. You can copy it somewhere in your PATH if you want to use it from anywhere.

As well is this package available at [crates.io](https://crates.io/crates/https-wrapper).
)

## Usage

### CLI (and providing certificate information)
The default way to use it is to provide three (or four) positional input parameters.
```bash
# Using PFX/PKCS12
https-wrapper <input-address> <output-address> <certificate.pfx> [<password>]
# Using PEM
https-wrapper <input-address> <output-address> <certificate.pem> <private-key.pem>
```
When using positional arguments for certificate information it is necessary to have the appropriate file extensions (either `.pfx`/`.p12` for PFX, or `.pem`/`.crt`/`.cer`/`.cert` for PEM certificates, and `.pem`/`.key` for private keys).

Alternatively the certificate information can be provided by named parameters:
```bash
# Using PFX/PKCS12
https-wrapper <input-address> <output-address> --pfx <certificate.pfx> [--password <password>]
# Using PEM
https-wrapper <input-address> <output-address> --cert <certificate.pem> --key <private-key.pem>
```
This approach is more explicit and does not demand a specific file extension.

### URL redirection
There is no default IP or port.

Probably the most common choice for input address is `0.0.0.0:443`, as `0.0.0.0` usually accepts remote incoming responses, and the internet expects to connect to port 443 for HTTPS.

To me the most obvious output address is something like `127.0.0.1:10000`, since `127.0.0.1` usually corresponds to localhost and the HTTP server to connect the TLS layer with runs at an arbitrary port like `10000`. 

Note: Picking port `80` for the HTTP server is probably _not_ what you want, because then clients have an unencrypted communication channel with the website. Nowadays most browsers will deny connection to such websites by default. Instead what you probably want to do is have any URL requests to port 80 (which is HTTP) to be redirected to HTTPS at port 443. I've built a tool for that as well [http-to-https-redirect](https://github.com/jvtubergen/http-to-https-redirect) that you might consider useful for this task.

## Using Let's Encrypt Certificates

This tool supports [Let's Encrypt](https://letsencrypt.org/) certificates in both PEM and PFX formats.

### Option 1: Use PEM files directly (recommended)
```bash
https-wrapper 0.0.0.0:443 127.0.0.1:8080 fullchain.pem privkey.pem
```

### Option 2: Convert to PFX format
If you prefer using PFX format, convert your Let's Encrypt certificate:

```bash
openssl pkcs12 -export -out cert.pfx -inkey privkey.pem -in fullchain.pem
```

You'll be prompted to set an export password, which you'll then use when running the https-wrapper:

```bash
https-wrapper 0.0.0.0:443 127.0.0.1:8080 cert.pfx yourpassword
```

## Architecture

![Architecture Diagram](documentation/diagram.svg)

The proxy operates in five key steps:

1. Accept incoming HTTPS connections on a specified port
2. Perform TLS handshake and decrypt traffic
3. Forward raw TCP bytes to a backend HTTP server
4. Encrypt responses and send them back to clients
5. Support any HTTP protocol version (HTTP/1.1, HTTP/2, WebSockets, etc.)

## Acknowledgements

Certificate handling implementation was inspired by [forge](https://github.com/nhudson/forge).