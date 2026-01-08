# Errors & Fixes

A log of all compilation and runtime errors encountered while building this QUIC/HTTP3 implementation, along with their solutions.

---

## 1. Missing Imports

### Error
```
error[E0425]: cannot find value `cert` in this scope
error[E0433]: failed to resolve: use of undeclared type `ServerConfig`
error[E0433]: failed to resolve: use of undeclared type `Arc`
error[E0433]: failed to resolve: use of undeclared type `Endpoint`
error[E0433]: failed to resolve: use of undeclared type `Bytes`
```

### Problem
Rust types were used without importing them.

### Fix
Added necessary imports at the top of the file:
```rust
use std::sync::Arc;
use bytes::Bytes;
use h3_quinn::quinn;
use quinn::{Endpoint, ServerConfig};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
```

---

## 2. Typo: `cart` vs `cert`

### Error
```
error[E0425]: cannot find value `cert` in this scope
   |
10 |         .with_single_cert(cart.cert_chain, cert.private_key)?;
   |                                            ^^^^ help: a local variable with a similar name exists: `cart`
```

### Problem
Variable was named `cart` but referenced as `cert`.

### Fix
Renamed to consistent `cert`:
```rust
let cert = generate_self_signed_cert()?;
// ...use cert.cert_chain, cert.private_key
```

---

## 3. Wrong Type Name: `TlsServerConfig`

### Error
```
error[E0433]: failed to resolve: use of undeclared type `TlsServerConfig`
```

### Problem
`TlsServerConfig` doesn't exist. The correct type is `rustls::ServerConfig`.

### Fix
```rust
// Before (wrong)
let mut tls_config = TlsServerConfig::builder()

// After (correct)
let mut tls_config = rustls::ServerConfig::builder()
```

---

## 4. Wrong h3 Module Path

### Error
```
error[E0433]: failed to resolve: could not find `quinn` in `h3`
   |
24 |             h3::server::Connection::new(h3::quinn::Connection::new(conn))
   |                                            ^^^^^ could not find `quinn` in `h3`
```

### Problem
`h3` doesn't have a `quinn` submodule. The QUIC adapter is a separate crate.

### Fix
```rust
// Before (wrong)
h3::quinn::Connection::new(conn)

// After (correct)
h3_quinn::Connection::new(conn)
```

---

## 5. Result Type Missing Error Parameter

### Error
```
error[E0107]: enum takes 2 generic arguments but 1 generic argument was supplied
   |
69 | fn generate_self_signed_cert() -> Result<CertificateChain> {
   |                                   ^^^^^^ expected 2 generic arguments
```

### Problem
`std::result::Result` requires both `Ok` and `Err` types.

### Fix
Use `anyhow::Result` which defaults the error type:
```rust
fn generate_self_signed_cert() -> anyhow::Result<CertificateChain> {
```

Also make `main()` return `Result`:
```rust
async fn main() -> anyhow::Result<()> {
```

---

## 6. `?` Operator in Non-Result Function

### Error
```
error[E0277]: the `?` operator can only be used in an async block that returns `Result` or `Option`
   |
22 |         let conn = conn.await?;
   |                              ^ cannot use the `?` operator in an async block that returns `()`
```

### Problem
Using `?` requires the function to return `Result` or `Option`.

### Fix
Changed main's return type:
```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // ... code with ? operators
    Ok(())
}
```

---

## 7. Endpoint::server Expects SocketAddr

### Error
```
error: mismatched types
  --> expected `SocketAddr`, found `&str`
```

### Problem
`Endpoint::server` expects a parsed `SocketAddr`, not a string.

### Fix
```rust
// Before (wrong)
let endpoint = Endpoint::server(server_config, "127.0.0.1:4433");

// After (correct)
let endpoint = Endpoint::server(server_config, "127.0.0.1:4433".parse()?)?;
```

---

## 8. h3 `accept()` Returns RequestResolver

### Error
```
error[E0308]: mismatched types
   |
36 | Ok(Some((req, mut stream))) => {
   |         ^^^^^^^^^^^^^^^^^ expected `RequestResolver`, found `(_, _)`
```

### Problem
In h3 0.0.8+, `accept()` returns a `RequestResolver`, not the request directly.

### Fix
```rust
// Before (wrong)
Ok(Some((req, mut stream))) => {
    // use req and stream directly
}

// After (correct)
Ok(Some(resolver)) => {
    let (req, mut stream) = resolver.resolve_request().await.unwrap();
    // now use req and stream
}
```

---

## 9. rcgen API Changed: `key_pair` → `signing_key`

### Error
```
error[E0609]: no field `key_pair` on type `CertifiedKey<...>`
   |
   = note: available fields are: `cert`, `signing_key`
```

### Problem
rcgen 0.14 renamed `key_pair` to `signing_key`.

### Fix
```rust
// Before (wrong)
certified_key.key_pair.serialize_der()

// After (correct)
certified_key.signing_key.serialize_der()
```

---

## 10. rcgen API Changed: `cert_der()` → `cert.der()`

### Error
```
error[E0599]: no method named `cert_der` found for struct `CertifiedKey`
```

### Problem
Method was renamed in rcgen 0.14.

### Fix
```rust
// Before (wrong)
vec![cert.cert_der().clone()]

// After (correct)
vec![certified_key.cert.der().clone()]
```

---

## 11. Buf Trait Not in Scope

### Error
```
error[E0599]: no method named `chunk` found for opaque type `impl Buf`
   |
   = help: items from traits can only be used if the trait is in scope
```

### Problem
The `chunk()` method comes from the `Buf` trait which wasn't imported.

### Fix
```rust
use bytes::Buf;
```

---

## 12. ALPN Protocol Mismatch (Runtime Error)

### Error
```
Error: aborted by peer: the cryptographic handshake failed: error 120: peer doesn't support any known protocol
```

### Problem
Server advertises `h3` ALPN protocol, but client didn't set any ALPN protocols.

### Fix
Add ALPN to client TLS config:
```rust
let mut tls_config = rustls::ClientConfig::builder()
    .dangerous()
    .with_custom_certificate_verifier(Arc::new(SkipServerVerification))
    .with_no_client_auth();

// This was missing!
tls_config.alpn_protocols = vec![b"h3".to_vec()];
```

**Explanation:** ALPN (Application-Layer Protocol Negotiation) is a TLS extension that allows client and server to agree on which application protocol to use. For HTTP/3, both must agree on `h3`.

---

## Summary

| Error Type | Count |
|------------|-------|
| Missing imports | 5 |
| Typos | 1 |
| Wrong type names | 2 |
| API changes (rcgen, h3) | 3 |
| Missing trait imports | 1 |
| Type inference issues | 2 |
| Protocol mismatch (runtime) | 1 |

**Total: 15 errors fixed**
