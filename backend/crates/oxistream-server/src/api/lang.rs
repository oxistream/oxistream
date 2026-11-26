use std::convert::Infallible;
use std::str::FromStr;

use axum::extract::FromRequestParts;
use axum::http::header::ACCEPT_LANGUAGE;
use axum::http::request::Parts;
use rustypipe::param::Language;

/// Content language for YouTube requests, taken from `Accept-Language` (default English).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContentLanguage(pub Language);

impl<S: Send + Sync> FromRequestParts<S> for ContentLanguage {
    type Rejection = Infallible;

    fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        let lang = parts
            .headers
            .get(ACCEPT_LANGUAGE)
            .and_then(|value| value.to_str().ok())
            .map_or(Language::En, parse_accept_language);
        std::future::ready(Ok(Self(lang)))
    }
}

pub fn parse_accept_language(header: &str) -> Language {
    let mut tags: Vec<(&str, f32)> = header
        .split(',')
        .filter_map(|entry| {
            let mut parts = entry.split(';');
            let tag = parts.next()?.trim();
            let quality = parts
                .find_map(|p| p.trim().strip_prefix("q="))
                .map_or(Some(1.0), |q| q.trim().parse::<f32>().ok())?;
            (!tag.is_empty() && tag != "*" && quality > 0.0).then_some((tag, quality))
        })
        .collect();
    tags.sort_by(|a, b| b.1.total_cmp(&a.1));

    tags.into_iter()
        .find_map(|(tag, _)| {
            Language::from_str(tag)
                .or_else(|_| Language::from_str(&tag.to_ascii_lowercase()))
                .ok()
        })
        .unwrap_or(Language::En)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_supported_languages() {
        assert_eq!(parse_accept_language("de"), Language::De);
        assert_eq!(
            parse_accept_language("de-DE,de;q=0.9,en;q=0.8"),
            Language::De
        );
        assert_eq!(parse_accept_language("en-US,en;q=0.9"), Language::En);
        assert_eq!(parse_accept_language("en-GB"), Language::EnGb);
        assert_eq!(parse_accept_language("DE-AT"), Language::De);
    }

    #[test]
    fn respects_quality_values() {
        assert_eq!(parse_accept_language("en;q=0.5, de;q=0.9"), Language::De);
        assert_eq!(parse_accept_language("de;q=0, en"), Language::En);
    }

    #[test]
    fn falls_back_to_english() {
        assert_eq!(parse_accept_language(""), Language::En);
        assert_eq!(parse_accept_language("*"), Language::En);
        assert_eq!(parse_accept_language("xx-YY, ;q=abc"), Language::En);
    }
}
