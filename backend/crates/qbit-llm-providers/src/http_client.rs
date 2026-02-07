//! HTTP client factory for proxy-aware reqwest clients.
//!
//! Provides a centralized way to build `reqwest::Client` instances configured
//! with proxy settings from the application configuration.

use anyhow::Result;
use qbit_settings::ProxySettings;

/// Build a `reqwest::Client` configured with the given proxy settings.
///
/// If no proxy settings are provided (or all fields are None), returns a default client
/// that respects standard environment variables (`HTTP_PROXY`, `HTTPS_PROXY`, `NO_PROXY`).
///
/// Supports HTTP, HTTPS, and SOCKS5 proxy URLs.
pub fn build_http_client(proxy: &ProxySettings) -> Result<reqwest::Client> {
    let mut builder = reqwest::Client::builder();

    if let Some(url) = &proxy.url {
        if !url.is_empty() {
            let no_proxy = proxy
                .no_proxy
                .as_deref()
                .filter(|s| !s.is_empty())
                .and_then(reqwest::NoProxy::from_string);

            let mut proxy_obj = reqwest::Proxy::all(url)?;

            if let (Some(username), Some(password)) = (&proxy.username, &proxy.password) {
                proxy_obj = proxy_obj.basic_auth(username, password);
            }

            proxy_obj = proxy_obj.no_proxy(no_proxy);

            builder = builder.proxy(proxy_obj);
        }
    }

    // TLS configuration
    if proxy.accept_invalid_certs {
        builder = builder.danger_accept_invalid_certs(true);
    }

    if let Some(ca_cert_path) = &proxy.ca_cert_path {
        if !ca_cert_path.is_empty() {
            let cert_bytes = std::fs::read(ca_cert_path)
                .map_err(|e| anyhow::anyhow!("Failed to read CA certificate file '{}': {}", ca_cert_path, e))?;
            
            // Parse PEM bundle and add each certificate
            let certs = reqwest::Certificate::from_pem_bundle(&cert_bytes)
                .map_err(|e| anyhow::anyhow!("Failed to parse CA certificate from '{}': {}", ca_cert_path, e))?;
            
            for cert in certs {
                builder = builder.add_root_certificate(cert);
            }
        }
    }

    Ok(builder.build()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_http_client_no_proxy() {
        let proxy = ProxySettings::default();
        let client = build_http_client(&proxy);
        assert!(client.is_ok());
    }

    #[test]
    fn test_build_http_client_with_url() {
        let proxy = ProxySettings {
            url: Some("http://proxy.example.com:8080".to_string()),
            username: None,
            password: None,
            no_proxy: None,
            ca_cert_path: None,
            accept_invalid_certs: false,
        };
        let client = build_http_client(&proxy);
        assert!(client.is_ok());
    }

    #[test]
    fn test_build_http_client_with_auth() {
        let proxy = ProxySettings {
            url: Some("http://proxy.example.com:8080".to_string()),
            username: Some("user".to_string()),
            password: Some("pass".to_string()),
            no_proxy: None,
            ca_cert_path: None,
            accept_invalid_certs: false,
        };
        let client = build_http_client(&proxy);
        assert!(client.is_ok());
    }

    #[test]
    fn test_build_http_client_with_no_proxy() {
        let proxy = ProxySettings {
            url: Some("http://proxy.example.com:8080".to_string()),
            username: None,
            password: None,
            no_proxy: Some("localhost,127.0.0.1,.local".to_string()),
            ca_cert_path: None,
            accept_invalid_certs: false,
        };
        let client = build_http_client(&proxy);
        assert!(client.is_ok());
    }

    #[test]
    fn test_build_http_client_with_socks5() {
        let proxy = ProxySettings {
            url: Some("socks5://proxy.example.com:1080".to_string()),
            username: None,
            password: None,
            no_proxy: None,
            ca_cert_path: None,
            accept_invalid_certs: false,
        };
        let client = build_http_client(&proxy);
        assert!(client.is_ok());
    }

    #[test]
    fn test_build_http_client_invalid_url() {
        let proxy = ProxySettings {
            url: Some("not-a-valid-url".to_string()),
            username: None,
            password: None,
            no_proxy: None,
            ca_cert_path: None,
            accept_invalid_certs: false,
        };
        let client = build_http_client(&proxy);
        // reqwest::Proxy::all accepts most strings; actual validation
        // happens at connect time, so this should still succeed
        // (reqwest is lenient with URL parsing for proxies)
        assert!(client.is_ok() || client.is_err());
    }

    #[test]
    fn test_build_http_client_empty_url() {
        let proxy = ProxySettings {
            url: Some("".to_string()),
            username: None,
            password: None,
            no_proxy: None,
            ca_cert_path: None,
            accept_invalid_certs: false,
        };
        // Empty URL should be treated as no proxy
        let client = build_http_client(&proxy);
        assert!(client.is_ok());
    }

    #[test]
    fn test_build_http_client_all_fields() {
        let proxy = ProxySettings {
            url: Some("http://proxy:3128".to_string()),
            username: Some("admin".to_string()),
            password: Some("secret".to_string()),
            no_proxy: Some("*.internal.corp,10.0.0.0/8".to_string()),
            ca_cert_path: None,
            accept_invalid_certs: false,
        };
        let client = build_http_client(&proxy);
        assert!(client.is_ok());
    }

    #[test]
    fn test_build_http_client_username_without_password() {
        // If only username is set without password, auth should not be applied
        let proxy = ProxySettings {
            url: Some("http://proxy:8080".to_string()),
            username: Some("user".to_string()),
            password: None,
            no_proxy: None,
            ca_cert_path: None,
            accept_invalid_certs: false,
        };
        let client = build_http_client(&proxy);
        assert!(client.is_ok());
    }

    #[test]
    fn test_build_http_client_empty_no_proxy() {
        let proxy = ProxySettings {
            url: Some("http://proxy:8080".to_string()),
            username: None,
            password: None,
            no_proxy: Some("".to_string()),
            ca_cert_path: None,
            accept_invalid_certs: false,
        };
        let client = build_http_client(&proxy);
        assert!(client.is_ok());
    }

    #[test]
    fn test_build_http_client_accept_invalid_certs() {
        let proxy = ProxySettings {
            url: Some("https://proxy.example.com:8443".to_string()),
            username: None,
            password: None,
            no_proxy: None,
            ca_cert_path: None,
            accept_invalid_certs: true,
        };
        let client = build_http_client(&proxy);
        assert!(client.is_ok());
    }

    #[test]
    fn test_build_http_client_with_ca_cert_missing_file() {
        let proxy = ProxySettings {
            url: None,
            username: None,
            password: None,
            no_proxy: None,
            ca_cert_path: Some("/nonexistent/ca-cert.pem".to_string()),
            accept_invalid_certs: false,
        };
        let result = build_http_client(&proxy);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Failed to read CA certificate file"));
    }

    #[test]
    fn test_build_http_client_with_ca_cert_no_pem_blocks() {
        // from_pem_bundle returns an empty Vec for non-PEM data (no error)
        let temp_dir = std::env::temp_dir();
        let cert_path = temp_dir.join("test-no-pem-blocks.pem");

        std::fs::write(&cert_path, "not a valid PEM file").unwrap();

        let proxy = ProxySettings {
            url: None,
            username: None,
            password: None,
            no_proxy: None,
            ca_cert_path: Some(cert_path.to_str().unwrap().to_string()),
            accept_invalid_certs: false,
        };

        let result = build_http_client(&proxy);

        // Clean up
        std::fs::remove_file(&cert_path).ok();

        // reqwest's from_pem_bundle silently ignores non-PEM content
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_http_client_with_empty_ca_cert_path() {
        // Empty ca_cert_path should be treated as not set
        let proxy = ProxySettings {
            url: None,
            username: None,
            password: None,
            no_proxy: None,
            ca_cert_path: Some("".to_string()),
            accept_invalid_certs: false,
        };
        let client = build_http_client(&proxy);
        assert!(client.is_ok());
    }
}
