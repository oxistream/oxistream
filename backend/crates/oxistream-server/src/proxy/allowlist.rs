use reqwest::Url;

use super::Kind;

const STREAM_SUFFIXES: &[&str] = &[".googlevideo.com"];
const IMAGE_HOSTS: &[&str] = &["yt3.googleusercontent.com"];
const IMAGE_SUFFIXES: &[&str] = &[".ytimg.com", ".ggpht.com"];
const SUBTITLE_SUFFIXES: &[&str] = &[".youtube.com"];

/// Upstream hosts the proxy may fetch from, per kind. Only HTTPS on the default port.
#[derive(Debug, Clone, Default)]
pub struct Allowlist {
    test_origins: Vec<String>,
}

impl Allowlist {
    pub fn youtube() -> Self {
        Self::default()
    }

    /// Also allows `origin` (e.g. `http://127.0.0.1:1234`) for every kind, for tests against a
    /// local mock upstream.
    #[must_use]
    pub fn allow_test_origin(mut self, origin: &Url) -> Self {
        self.test_origins
            .push(origin.origin().ascii_serialization());
        self
    }

    pub fn allows(&self, kind: Kind, url: &Url) -> bool {
        if self
            .test_origins
            .iter()
            .any(|o| *o == url.origin().ascii_serialization())
        {
            return true;
        }
        if url.scheme() != "https" || url.port().is_some() || !url.username().is_empty() {
            return false;
        }
        let Some(host) = url.host_str() else {
            return false;
        };
        let (hosts, suffixes) = match kind {
            Kind::Stream => (&[][..], STREAM_SUFFIXES),
            Kind::Image => (IMAGE_HOSTS, IMAGE_SUFFIXES),
            Kind::Subtitle => (&[][..], SUBTITLE_SUFFIXES),
        };
        hosts.contains(&host) || suffixes.iter().any(|suffix| host.ends_with(suffix))
    }

    pub fn allows_any(&self, url: &Url) -> bool {
        [Kind::Stream, Kind::Image, Kind::Subtitle]
            .into_iter()
            .any(|kind| self.allows(kind, url))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn allows(kind: Kind, url: &str) -> bool {
        Allowlist::youtube().allows(kind, &Url::parse(url).unwrap())
    }

    #[test]
    fn allows_youtube_hosts_per_kind() {
        assert!(allows(
            Kind::Stream,
            "https://rr1---sn-abc.googlevideo.com/videoplayback?itag=251"
        ));
        assert!(allows(
            Kind::Image,
            "https://i.ytimg.com/vi/x/hqdefault.jpg"
        ));
        assert!(allows(
            Kind::Image,
            "https://i9.ytimg.com/sb/x/storyboard.jpg"
        ));
        assert!(allows(Kind::Image, "https://yt3.ggpht.com/abc=s88"));
        assert!(allows(Kind::Image, "https://yt3.googleusercontent.com/abc"));
        assert!(allows(
            Kind::Subtitle,
            "https://www.youtube.com/api/timedtext?v=x&lang=en"
        ));
    }

    #[test]
    fn rejects_hosts_of_other_kinds() {
        assert!(!allows(
            Kind::Image,
            "https://rr1.googlevideo.com/videoplayback"
        ));
        assert!(!allows(
            Kind::Stream,
            "https://i.ytimg.com/vi/x/hqdefault.jpg"
        ));
        assert!(!allows(
            Kind::Stream,
            "https://www.youtube.com/api/timedtext"
        ));
        assert!(!allows(Kind::Image, "https://www.youtube.com/favicon.ico"));
    }

    #[test]
    fn rejects_lookalikes_and_non_https() {
        for url in [
            "http://rr1.googlevideo.com/videoplayback",
            "https://rr1.googlevideo.com:8443/videoplayback",
            "https://googlevideo.com.evil.com/videoplayback",
            "https://evilgooglevideo.com/videoplayback",
            "https://user@rr1.googlevideo.com/videoplayback",
            "https://127.0.0.1/videoplayback",
            "file:///etc/passwd",
        ] {
            assert!(!allows(Kind::Stream, url), "{url}");
        }
        assert!(!allows(Kind::Image, "https://ytimg.com.evil.com/x.jpg"));
        assert!(!allows(
            Kind::Subtitle,
            "https://youtube.com.evil.com/api/timedtext"
        ));
    }

    #[test]
    fn test_origin_is_exact() {
        let origin = Url::parse("http://127.0.0.1:4000").unwrap();
        let list = Allowlist::youtube().allow_test_origin(&origin);
        let ok = Url::parse("http://127.0.0.1:4000/videoplayback").unwrap();
        let other_port = Url::parse("http://127.0.0.1:4001/videoplayback").unwrap();
        assert!(list.allows(Kind::Stream, &ok));
        assert!(!list.allows(Kind::Stream, &other_port));
        assert!(!Allowlist::youtube().allows(Kind::Stream, &ok));
    }
}
