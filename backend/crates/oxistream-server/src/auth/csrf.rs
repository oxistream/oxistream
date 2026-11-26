//! Origin check for state-changing requests.

use axum::http::{HeaderMap, Method, Uri, header};
use reqwest::Url;

pub fn is_safe_method(method: &Method) -> bool {
    matches!(*method, Method::GET | Method::HEAD | Method::OPTIONS)
}

/// Whether the request's `Origin` (or, without one, its `Referer`) is this instance: the
/// origin named by the `Host` header, or `public_origin` if configured.
pub fn origin_allowed(headers: &HeaderMap, uri: &Uri, public_origin: Option<&str>) -> bool {
    let Some(origin) = request_origin(headers) else {
        return false;
    };
    let serialized = origin.origin().ascii_serialization();
    if public_origin == Some(serialized.as_str()) {
        return true;
    }
    let host = headers
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        .or_else(|| uri.authority().map(axum::http::uri::Authority::as_str));
    host.and_then(|host| Url::parse(&format!("{}://{host}", origin.scheme())).ok())
        .is_some_and(|url| url.path() == "/" && url.origin().ascii_serialization() == serialized)
}

/// The URL in `Origin`, or in `Referer` without one, if it has a tuple origin.
fn request_origin(headers: &HeaderMap) -> Option<Url> {
    let header = |name| headers.get(name).and_then(|value| value.to_str().ok());
    let value = match header(header::ORIGIN) {
        Some(origin) => origin,
        None => header(header::REFERER)?,
    };
    Url::parse(value).ok().filter(|url| url.origin().is_tuple())
}

#[cfg(test)]
mod tests {
    use axum::http::HeaderValue;

    use super::*;

    fn allowed(pairs: &[(&'static str, &'static str)], public: Option<&str>) -> bool {
        let mut headers = HeaderMap::new();
        for (name, value) in pairs {
            headers.insert(*name, HeaderValue::from_static(value));
        }
        origin_allowed(&headers, &Uri::from_static("/api/v0/auth/login"), public)
    }

    #[test]
    fn same_host_origin_is_allowed() {
        assert!(allowed(
            &[
                ("origin", "http://localhost:5173"),
                ("host", "localhost:5173")
            ],
            None
        ));
        assert!(allowed(
            &[
                ("origin", "https://tube.example"),
                ("host", "Tube.Example:443")
            ],
            None
        ));
        assert!(allowed(
            &[
                ("referer", "https://tube.example/login?redirect=/"),
                ("host", "tube.example")
            ],
            None
        ));
    }

    #[test]
    fn foreign_or_missing_origin_is_rejected() {
        assert!(!allowed(&[("host", "tube.example")], None));
        assert!(!allowed(
            &[("origin", "https://evil.example"), ("host", "tube.example")],
            None
        ));
        assert!(!allowed(
            &[
                ("origin", "https://tube.example:8443"),
                ("host", "tube.example")
            ],
            None
        ));
        assert!(!allowed(
            &[("origin", "null"), ("host", "tube.example")],
            None
        ));
        assert!(!allowed(
            &[
                ("origin", "https://evil.example"),
                ("host", "tube.example/x")
            ],
            None
        ));
        assert!(!allowed(&[("origin", "https://tube.example")], None));
    }

    #[test]
    fn public_origin_is_allowed_behind_proxies() {
        let public = Some("https://tube.example");
        assert!(allowed(
            &[
                ("origin", "https://tube.example"),
                ("host", "oxistream:8080")
            ],
            public
        ));
        assert!(!allowed(
            &[
                ("origin", "http://tube.example"),
                ("host", "oxistream:8080")
            ],
            public
        ));
    }
}
