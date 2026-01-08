use std::sync::Arc;

use bytes::Buf;
use h3_quinn::quinn;
use http::Request;
use quinn::Endpoint;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    rustls::crypto::aws_lc_rs::default_provider().install_default().unwrap();

    // Configure client to accept self-signed certificates (for development)
    let mut tls_config = rustls::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(SkipServerVerification))
        .with_no_client_auth();
    
    // Must match server's ALPN protocol for HTTP/3
    tls_config.alpn_protocols = vec![b"h3".to_vec()];

    let client_config = quinn::ClientConfig::new(Arc::new(
        quinn::crypto::rustls::QuicClientConfig::try_from(tls_config)?
    ));

    let mut endpoint = Endpoint::client("0.0.0.0:0".parse()?)?;
    endpoint.set_default_client_config(client_config);

    println!("Connecting to server at 127.0.0.1:4433...");

    let conn = endpoint
        .connect("127.0.0.1:4433".parse()?, "localhost")?
        .await?;

    println!("Connected! Establishing HTTP/3 connection...");

    let (mut driver, mut send_request) = h3::client::new(h3_quinn::Connection::new(conn)).await?;

    // Spawn driver to handle connection
    tokio::spawn(async move {
        futures::future::poll_fn(|cx| driver.poll_close(cx)).await;
    });

    // Test different endpoints
    for path in &["/", "/test", "/health", "/unknown"] {
        println!("\n--- Requesting {} ---", path);
        
        let req = Request::builder()
            .method("GET")
            .uri(format!("https://localhost{}", path))
            .body(())?;

        let mut stream = send_request.send_request(req).await?;
        stream.finish().await?;

        let response = stream.recv_response().await?;
        println!("Status: {}", response.status());
        
        // Read response body
        let mut body = Vec::new();
        while let Some(chunk) = stream.recv_data().await? {
            body.extend(chunk.chunk());
        }
        println!("Body: {}", String::from_utf8_lossy(&body));
    }

    println!("\n✅ All requests completed successfully!");

    Ok(())
}

// Custom certificate verifier that skips verification (for self-signed certs in development)
#[derive(Debug)]
struct SkipServerVerification;

impl rustls::client::danger::ServerCertVerifier for SkipServerVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::RSA_PKCS1_SHA384,
            rustls::SignatureScheme::RSA_PKCS1_SHA512,
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
            rustls::SignatureScheme::ECDSA_NISTP521_SHA512,
            rustls::SignatureScheme::RSA_PSS_SHA256,
            rustls::SignatureScheme::RSA_PSS_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA512,
            rustls::SignatureScheme::ED25519,
        ]
    }
}
