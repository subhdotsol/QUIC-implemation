# QUIC Demo

A demonstration of building a **QUIC server and client** in Rust, with HTTP/3 support.

## What is QUIC?

QUIC (Quick UDP Internet Connections) is a modern transport layer protocol that provides:
- **Faster connections** — 0-RTT and 1-RTT handshakes reduce latency
- **Built-in encryption** — TLS 1.3 is mandatory, no separate TLS handshake
- **Multiplexing without head-of-line blocking** — Multiple streams over a single connection
- **Connection migration** — Seamlessly handle network changes (WiFi → cellular)

## How We're Building It

### Architecture

```
┌─────────────┐                    ┌─────────────┐
│   Client    │◄──── QUIC/H3 ────►│   Server    │
│ (client.rs) │                    │ (main.rs)   │
└─────────────┘                    └─────────────┘
```

### Technology Stack

| Layer | Library | Purpose |
|-------|---------|---------|
| **Runtime** | `tokio` | Async runtime for non-blocking I/O |
| **Transport** | `quinn` | Pure Rust QUIC implementation |
| **Security** | `rustls` + `rcgen` | TLS 1.3 encryption & certificate generation |
| **Application** | `h3` + `h3-quinn` | HTTP/3 protocol over QUIC |
| **HTTP Types** | `http` | Standard HTTP request/response types |

### Key Components

1. **quinn** — Provides the core QUIC transport layer, handling:
   - Connection establishment
   - Stream multiplexing
   - Congestion control
   - Loss recovery

2. **rustls** — A pure Rust TLS implementation that provides:
   - TLS 1.3 handshake
   - Certificate validation
   - Secure key exchange

3. **rcgen** — Generates self-signed X.509 certificates for development

4. **h3 + h3-quinn** — HTTP/3 implementation that runs on top of QUIC:
   - QPACK header compression
   - HTTP semantics over QUIC streams

## Running the Demo

### Start the Server
```bash
cargo run --bin server
```

### Start the Client
```bash
cargo run --bin client
```

## Project Structure

```
QUIC-demo/
├── src/
│   ├── main.rs      # QUIC server implementation
│   └── client.rs    # QUIC client implementation
├── Cargo.toml       # Dependencies with explanations
└── README.md        # This file
```

## Resources

- [QUIC RFC 9000](https://www.rfc-editor.org/rfc/rfc9000.html)
- [HTTP/3 RFC 9114](https://www.rfc-editor.org/rfc/rfc9114.html)
- [quinn documentation](https://docs.rs/quinn)
- [h3 documentation](https://docs.rs/h3)
