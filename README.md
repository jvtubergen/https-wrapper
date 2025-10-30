= HTTPS wrapper

_Minimalistic bash command to provide a HTTPS layer to your HTTP server._

This code constructs a reverse proxy to provide TLS connections and forward any requests to another port (there where you have your HTTP server listening at).

Provide the following:
- What address (ip:port) this (HTTPS reverse proxy) server should listen to.
- What address (ip:port) this server should forward the request information to.
- The TLS certificate with its password. (Supports both `.pfx` and `.chain` + `.key`).

[https server (ip:port + certificate)] -> [http server (ip:port)]

Usage:
`https-wrapper <input-address> <output-address> <certificate> <password>`

Example:
`https-wrapper 0.0.0.0:443 127.0.0.1:8080 cert.pfx mypassword`

## Using Let's Encrypt Certificates

This tool has been tested with Let's Encrypt certificates converted to PFX format. To convert your Let's Encrypt certificate:

```bash
openssl pkcs12 -export -out cert.pfx -inkey key.pem -in cert.pem
```

You'll be prompted to set an export password, which you'll then use when running the https-wrapper.