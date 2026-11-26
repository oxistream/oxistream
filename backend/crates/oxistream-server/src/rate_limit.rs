//! Per-client-IP rate limiting for `/api` and `/proxy`.

use std::net::{IpAddr, Ipv6Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{HeaderName, HeaderValue, Request, Response, StatusCode, header};
use governor::middleware::NoOpMiddleware;
use tower_governor::GovernorError;
use tower_governor::governor::{GovernorConfig, GovernorConfigBuilder};
use tower_governor::key_extractor::KeyExtractor;

use crate::config::RateLimitConfig;

const CLEANUP_INTERVAL: Duration = Duration::from_mins(1);

pub type Layer = tower_governor::GovernorLayer<ClientIp, NoOpMiddleware, Body>;

#[derive(Clone)]
pub struct RateLimit {
    config: Arc<GovernorConfig<ClientIp, NoOpMiddleware>>,
}

impl RateLimit {
    pub fn new(config: &RateLimitConfig) -> Self {
        let governor = GovernorConfigBuilder::default()
            .period(Duration::from_secs(1) / config.per_second.get())
            .burst_size(config.burst.get())
            .key_extractor(ClientIp {
                trusted_header: config.trusted_header.clone(),
            })
            .finish()
            .expect("period and burst size are non-zero");
        Self {
            config: Arc::new(governor),
        }
    }

    pub fn layer(&self) -> Layer {
        tower_governor::GovernorLayer::new(self.config.clone())
            .error_handler(|err| error_response(&err))
    }

    /// Periodically forgets idle clients so the limiter's memory stays bounded.
    pub fn spawn_cleanup(&self) {
        let limiter = self.config.limiter().clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(CLEANUP_INTERVAL);
            loop {
                interval.tick().await;
                limiter.retain_recent();
                limiter.shrink_to_fit();
            }
        });
    }
}

fn error_response(err: &GovernorError) -> Response<Body> {
    let mut response = Response::new(Body::empty());
    match err {
        GovernorError::TooManyRequests { wait_time, .. } => {
            *response.status_mut() = StatusCode::TOO_MANY_REQUESTS;
            *response.body_mut() = Body::from("too many requests");
            response
                .headers_mut()
                .insert(header::RETRY_AFTER, HeaderValue::from((*wait_time).max(1)));
        }
        GovernorError::UnableToExtractKey => {
            tracing::error!("rate limiter could not determine the client IP");
            *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
        }
        GovernorError::Other { code, .. } => *response.status_mut() = *code,
    }
    response
}

/// Keys requests by client IP: the trusted proxy header if configured, else the peer address.
/// IPv6 clients are grouped by /64, the smallest block usually assigned to one subscriber.
#[derive(Debug, Clone)]
pub struct ClientIp {
    trusted_header: Option<HeaderName>,
}

impl KeyExtractor for ClientIp {
    type Key = IpAddr;

    fn extract<T>(&self, req: &Request<T>) -> Result<Self::Key, GovernorError> {
        let forwarded = self.trusted_header.as_ref().and_then(|name| {
            let value = req.headers().get_all(name).iter().next_back()?;
            value.to_str().ok()?.rsplit(',').next()?.trim().parse().ok()
        });
        let peer = || {
            req.extensions()
                .get::<ConnectInfo<SocketAddr>>()
                .map(|ConnectInfo(addr)| addr.ip())
        };
        forwarded
            .or_else(peer)
            .map(normalize)
            .ok_or(GovernorError::UnableToExtractKey)
    }
}

fn normalize(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V4(_) => ip,
        IpAddr::V6(v6) => match v6.to_ipv4_mapped() {
            Some(v4) => IpAddr::V4(v4),
            None => IpAddr::V6(Ipv6Addr::from_bits(v6.to_bits() & !u128::from(u64::MAX))),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn extract(
        trusted: Option<&'static str>,
        headers: &[(&str, &str)],
        peer: Option<&str>,
    ) -> Option<IpAddr> {
        let mut req = Request::get("/").body(()).unwrap();
        for (name, value) in headers {
            req.headers_mut().append(
                HeaderName::from_bytes(name.as_bytes()).unwrap(),
                HeaderValue::from_str(value).unwrap(),
            );
        }
        if let Some(peer) = peer {
            req.extensions_mut()
                .insert(ConnectInfo::<SocketAddr>(peer.parse().unwrap()));
        }
        ClientIp {
            trusted_header: trusted.map(HeaderName::from_static),
        }
        .extract(&req)
        .ok()
    }

    fn ip(s: &str) -> IpAddr {
        s.parse().unwrap()
    }

    #[test]
    fn uses_peer_ip_by_default() {
        let headers = [("x-forwarded-for", "203.0.113.9")];
        assert_eq!(
            extract(None, &headers, Some("192.0.2.1:5000")),
            Some(ip("192.0.2.1"))
        );
        assert_eq!(extract(None, &headers, None), None);
    }

    #[test]
    fn uses_last_entry_of_trusted_header() {
        let headers = [
            ("x-forwarded-for", "198.51.100.1"),
            ("x-forwarded-for", "10.0.0.1, 203.0.113.9"),
        ];
        assert_eq!(
            extract(Some("x-forwarded-for"), &headers, Some("192.0.2.1:5000")),
            Some(ip("203.0.113.9"))
        );
        assert_eq!(
            extract(Some("x-forwarded-for"), &[], Some("192.0.2.1:5000")),
            Some(ip("192.0.2.1"))
        );
    }

    #[test]
    fn groups_ipv6_by_prefix() {
        assert_eq!(
            extract(None, &[], Some("[2001:db8:1:2:aaaa::1]:443")),
            Some(ip("2001:db8:1:2::"))
        );
        assert_eq!(
            extract(None, &[], Some("[::ffff:192.0.2.7]:443")),
            Some(ip("192.0.2.7"))
        );
    }
}
