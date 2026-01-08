use std::sync::Arc;

use bytes::Bytes;
use h3_quinn::quinn;
use quinn::{Endpoint, ServerConfig};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    rustls::crypto::aws_lc_rs::default_provider().install_default().unwrap();

    let cert = generate_self_signed_cert()?;
    let mut tls_config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(cert.cert_chain, cert.private_key)?;
    tls_config.alpn_protocols = vec![b"h3".to_vec()];

    let server_config = ServerConfig::with_crypto(Arc::new(
        quinn::crypto::rustls::QuicServerConfig::try_from(tls_config)?
    ));

    let endpoint = Endpoint::server(server_config, "127.0.0.1:4433".parse()?)?;

    println!("HTTP/3 server listening on 127.0.0.1:4433");

    while let Some(conn) = endpoint.accept().await {
        let conn = conn.await?;
        tokio::spawn(async move {
            let mut h3_conn: h3::server::Connection<h3_quinn::Connection, Bytes> = 
                h3::server::Connection::new(h3_quinn::Connection::new(conn))
                    .await
                    .unwrap();

            loop {
                match h3_conn.accept().await {
                    Ok(Some(resolver)) => {
                        tokio::spawn(async move {
                            // Resolve the request to get the actual request and stream
                            let (req, mut stream) = resolver.resolve_request().await.unwrap();
                            
                            println!("Got request for path: {}, protocol: {:?}", req.uri().path(), req.version());

                            let response_body: &str = match req.uri().path() {
                                "/" => "Hello from http3 server",
                                "/test" => "Hello from http3 test endpoint", 
                                "/health" => "hello from http3 health check",
                                _ => "404 Not Found", 
                            };

                            let response = http::Response::builder()
                                .status(http::StatusCode::OK)
                                .header("Content-Type", "text/plain")
                                .body(())
                                .unwrap();

                            stream.send_response(response).await.unwrap();
                            stream.send_data(Bytes::from(response_body)).await.unwrap();
                            stream.finish().await.unwrap();
                        });    
                    }
                    Ok(None) => break,
                    Err(_e) => break, 
                }
            }
        });
    }

    Ok(())
}

struct CertificateChain {
    cert_chain: Vec<CertificateDer<'static>>,
    private_key: PrivateKeyDer<'static>
}

// generate self signed certificate
fn generate_self_signed_cert() -> anyhow::Result<CertificateChain> {
    let certified_key = rcgen::generate_simple_self_signed(vec!["localhost".to_string()])?;
    let private_key_der = certified_key.signing_key.serialize_der();
    let private_key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(private_key_der));
    let cert_chain = vec![certified_key.cert.der().clone()];
    Ok(CertificateChain { cert_chain, private_key })
}
