mod blocking_http;
mod config;

use std::sync::Arc;

use rustls::{ClientConfig, RootCertStore};

pub use blocking_http::{BlockingHttpClient, BlockingHttpClientError, HttpResponse};
pub use config::HttpClientConfig;

pub(crate) fn tls_config() -> Arc<ClientConfig> {
    let mut roots = RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

    Arc::new(
        ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth(),
    )
}
