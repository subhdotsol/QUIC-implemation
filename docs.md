# QUIC + HTTP/3 Implementation Guide

A deep dive into building a QUIC server and client with HTTP/3 support in Rust.

---

## Architecture Overview

```
┌──────────────────────────────────────────────────────────────────┐
│                        Application Layer                         │
│                     (HTTP/3 via h3 crate)                        │
├──────────────────────────────────────────────────────────────────┤
│                        Transport Layer                           │
│                      (QUIC via quinn)                            │
├──────────────────────────────────────────────────────────────────┤
│                        Security Layer                            │
│                    (TLS 1.3 via rustls)                          │
├──────────────────────────────────────────────────────────────────┤
│                         UDP Transport                            │
└──────────────────────────────────────────────────────────────────┘
```

---

## Server Implementation (`src/main.rs`)

### 1. Crypto Provider Initialization

```rust
rustls::crypto::aws_lc_rs::default_provider().install_default().unwrap();
```

**Why?** rustls requires a cryptographic backend. We use AWS LC (libcrypto) which is fast and FIPS-compliant.

---

### 2. Self-Signed Certificate Generation

```rust
fn generate_self_signed_cert() -> anyhow::Result<CertificateChain> {
    let certified_key = rcgen::generate_simple_self_signed(vec!["localhost".to_string()])?;
    let private_key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(
        certified_key.signing_key.serialize_der()
    ));
    let cert_chain = vec![certified_key.cert.der().clone()];
    Ok(CertificateChain { cert_chain, private_key })
}
```

**What happens:**
1. `rcgen` generates a self-signed X.509 certificate for `localhost`
2. We extract the private key in PKCS#8 DER format
3. We get the certificate in DER format
4. Both are needed for TLS configuration

---

### 3. TLS Configuration

```rust
let mut tls_config = rustls::ServerConfig::builder()
    .with_no_client_auth()           // Don't require client certificates
    .with_single_cert(cert.cert_chain, cert.private_key)?;
tls_config.alpn_protocols = vec![b"h3".to_vec()];  // Advertise HTTP/3
```

**Key points:**
- `with_no_client_auth()` — Server doesn't verify client certificates
- `alpn_protocols = ["h3"]` — **Critical!** ALPN negotiates the application protocol. Both client and server must agree on `h3` for HTTP/3.

---

### 4. QUIC Server Configuration

```rust
let server_config = ServerConfig::with_crypto(Arc::new(
    quinn::crypto::rustls::QuicServerConfig::try_from(tls_config)?
));
```

This wraps the TLS config into a QUIC-compatible configuration. Quinn handles:
- Connection establishment
- Stream multiplexing
- Congestion control
- Packet loss recovery

---

### 5. Endpoint Creation

```rust
let endpoint = Endpoint::server(server_config, "127.0.0.1:4433".parse()?)?;
```

Creates a UDP socket bound to port 4433, ready to accept QUIC connections.

---

### 6. Connection Handling Loop

```rust
while let Some(conn) = endpoint.accept().await {
    let conn = conn.await?;
    tokio::spawn(async move {
        // Handle this connection in a new task
    });
}
```

**Flow:**
1. `endpoint.accept()` waits for incoming connections
2. Each connection is spawned into its own async task
3. This allows handling many concurrent connections

---

### 7. HTTP/3 Connection Setup

```rust
let mut h3_conn: h3::server::Connection<h3_quinn::Connection, Bytes> = 
    h3::server::Connection::new(h3_quinn::Connection::new(conn))
        .await
        .unwrap();
```

**What this does:**
- Wraps the QUIC connection (`conn`) in `h3_quinn::Connection`
- Creates an HTTP/3 server connection on top
- Type annotation is needed for the compiler

---

### 8. Request Handling

```rust
loop {
    match h3_conn.accept().await {
        Ok(Some(resolver)) => {
            tokio::spawn(async move {
                let (req, mut stream) = resolver.resolve_request().await.unwrap();
                // Process request...
            });    
        }
        Ok(None) => break,  // Connection closed gracefully
        Err(_e) => break,   // Connection error
    }
}
```

**Key concepts:**
- `accept()` returns a `RequestResolver`, not the request directly
- Must call `resolve_request()` to get the actual `Request` and `RequestStream`
- Each request is spawned into its own task for concurrency

---

### 9. Sending Response

```rust
let response = http::Response::builder()
    .status(http::StatusCode::OK)
    .header("Content-Type", "text/plain")
    .body(())
    .unwrap();

stream.send_response(response).await.unwrap();
stream.send_data(Bytes::from(response_body)).await.unwrap();
stream.finish().await.unwrap();
```

**Steps:**
1. Build response headers (note: `body(())` — headers only, no body yet)
2. `send_response()` — Send headers to client
3. `send_data()` — Send body data
4. `finish()` — Close the stream (signals end of response)

---

## Client Implementation (`src/client.rs`)

### 1. Custom Certificate Verifier

```rust
struct SkipServerVerification;

impl rustls::client::danger::ServerCertVerifier for SkipServerVerification {
    fn verify_server_cert(...) -> Result<ServerCertVerified, Error> {
        Ok(ServerCertVerified::assertion())  // Accept any cert
    }
    // ... other required methods
}
```

**Why?** Our server uses self-signed certificates. In production, you'd use proper CA-signed certs.

> ⚠️ **Never use this in production!** It disables all certificate validation.

---

### 2. Client TLS + ALPN Configuration

```rust
let mut tls_config = rustls::ClientConfig::builder()
    .dangerous()
    .with_custom_certificate_verifier(Arc::new(SkipServerVerification))
    .with_no_client_auth();

tls_config.alpn_protocols = vec![b"h3".to_vec()];  // Must match server!
```

**Critical:** The `alpn_protocols` must match what the server advertises, otherwise you get:
```
error 120: peer doesn't support any known protocol
```

---

### 3. Connection Establishment

```rust
let mut endpoint = Endpoint::client("0.0.0.0:0".parse()?)?;
endpoint.set_default_client_config(client_config);

let conn = endpoint
    .connect("127.0.0.1:4433".parse()?, "localhost")?  // SNI hostname
    .await?;
```

**Parameters:**
- `"0.0.0.0:0"` — Bind to any local address, let OS pick port
- `"127.0.0.1:4433"` — Server address
- `"localhost"` — Server Name Indication (SNI) for TLS

---

### 4. HTTP/3 Client Setup

```rust
let (mut driver, mut send_request) = h3::client::new(h3_quinn::Connection::new(conn)).await?;

// Driver must be polled to handle connection events
tokio::spawn(async move {
    futures::future::poll_fn(|cx| driver.poll_close(cx)).await;
});
```

**Two components:**
- `driver` — Handles connection-level events (must be polled!)
- `send_request` — Used to send HTTP requests

---

### 5. Sending Requests

```rust
let req = Request::builder()
    .method("GET")
    .uri("https://localhost/")
    .body(())?;

let mut stream = send_request.send_request(req).await?;
stream.finish().await?;  // Indicates no more request body
```

---

### 6. Receiving Response

```rust
let response = stream.recv_response().await?;
println!("Status: {}", response.status());

// Read body chunks
let mut body = Vec::new();
while let Some(chunk) = stream.recv_data().await? {
    body.extend(chunk.chunk());  // chunk() from Buf trait
}
```

---

## Dependency Roles

| Crate | Purpose |
|-------|---------|
| `quinn` | QUIC protocol implementation |
| `rustls` | TLS 1.3 library |
| `rcgen` | Certificate generation |
| `h3` | HTTP/3 protocol |
| `h3-quinn` | Glue between h3 and quinn |
| `http` | HTTP types (Request, Response) |
| `bytes` | Efficient byte buffers |
| `tokio` | Async runtime |
| `futures` | Async utilities |
| `anyhow` | Error handling |

---

## Data Flow

```
Client                                 Server
  │                                      │
  ├── QUIC handshake (UDP) ──────────────┤
  │   └── TLS 1.3 + ALPN "h3"            │
  │                                      │
  ├── HTTP/3 HEADERS frame ──────────────┤
  │   GET / HTTP/3                       │
  │                                      │
  │◄─────────── HTTP/3 HEADERS ──────────┤
  │             200 OK                   │
  │                                      │
  │◄─────────── HTTP/3 DATA ─────────────┤
  │             "Hello from..."          │
  │                                      │
  └── Stream FIN ────────────────────────┘
```
