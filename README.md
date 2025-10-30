= HTTPS wrapper

_Minimalistic bash command to provide a HTTPS layer to your HTTP server._

This code constructs a reverse proxy to provide TLS connections and forward any requests to another port (there where you have your HTTP server listening at).

Provide the following:
- What port this (HTTPS reverse proxy) server should listen to.
- What port this server should forward the request information to.
- The TLS certificate with its password. (Supports both `.pfx` and `.chain` + `.key`).

[https server (port + certificate)] -> [http server (port)] 

Usage:
`https-wrapper <input-port> <output-port> <certificate> <password>`