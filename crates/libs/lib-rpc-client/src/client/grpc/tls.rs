use std::sync::Arc;

/// Generate Rustls config
/// 
/// Default alpn protocols are  only `h2`
#[cfg(feature = "__rustls")]
pub(crate) fn rustls_config(danger_ignore_invalid_certs: bool) -> rustls::ClientConfig {
    let builder: rustls::ConfigBuilder<rustls::ClientConfig, rustls::WantsVerifier> =
        rustls::ClientConfig::builder().with_safe_defaults();

    let mut roots = rustls::RootCertStore::empty();
    roots.add_trust_anchors(webpki_roots::TLS_SERVER_ROOTS.iter().map(|ta| {
        tokio_rustls::rustls::OwnedTrustAnchor::from_subject_spki_name_constraints(
            ta.subject,
            ta.spki,
            ta.name_constraints,
        )
    }));

    let mut config = builder.with_root_certificates(roots).with_no_client_auth();

    config.alpn_protocols.push("h2".as_bytes().to_vec());

    if danger_ignore_invalid_certs {
        config
            .dangerous()
            .set_certificate_verifier(Arc::new(Verifier));
    }

    config
}

#[cfg(feature = "__rustls")]
pub struct Verifier;

#[cfg(feature = "__rustls")]
use rustls::client::ServerCertVerifier;

#[cfg(feature = "__rustls")]
impl ServerCertVerifier for Verifier {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::Certificate,
        _intermediates: &[rustls::Certificate],
        _server_name: &rustls::ServerName,
        _scts: &mut dyn Iterator<Item = &[u8]>,
        _ocsp_response: &[u8],
        _now: std::time::SystemTime,
    ) -> Result<rustls::client::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::Certificate,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::Certificate,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::HandshakeSignatureValid::assertion())
    }
}

// use rustls::client::danger::ServerCertVerifier;

// #[derive(Debug)]
// pub(crate) struct NoVerifier;
// impl ServerCertVerifier for NoVerifier {
//     fn verify_server_cert(
//         &self,
//         _end_entity: &rustls::pki_types::CertificateDer<'_>,
//         _intermediates: &[rustls::pki_types::CertificateDer<'_>],
//         _server_name: &rustls::pki_types::ServerName<'_>,
//         _ocsp_response: &[u8],
//         _now: rustls::pki_types::UnixTime,
//     ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
//         Ok(rustls::client::danger::ServerCertVerified::assertion())
//     }

//     fn verify_tls12_signature(
//         &self,
//         _message: &[u8],
//         _cert: &rustls::pki_types::CertificateDer<'_>,
//         _dss: &rustls::DigitallySignedStruct,
//     ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
//         Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
//     }

//     fn verify_tls13_signature(
//         &self,
//         _message: &[u8],
//         _cert: &rustls::pki_types::CertificateDer<'_>,
//         _dss: &rustls::DigitallySignedStruct,
//     ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
//         Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
//     }

//     fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
//         Vec::new()
//     }
// }
