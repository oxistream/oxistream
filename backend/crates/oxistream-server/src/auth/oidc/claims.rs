//! Usernames and groups from ID token or userinfo claims.

use serde_json::{Map, Value};

use crate::auth::password::{USERNAME_MAX_LENGTH, USERNAME_MIN_LENGTH};

pub type Claims = Map<String, Value>;

const FALLBACK_USERNAME: &str = "user";

/// The preferred username for a new account: the first of `preferred_username`, `name`,
/// the local part of `email` and `sub` that is still a valid username after sanitizing.
pub fn username_base(claims: &Claims) -> String {
    ["preferred_username", "name", "email", "sub"]
        .iter()
        .filter_map(|claim| claims.get(*claim)?.as_str())
        .find_map(sanitize)
        .unwrap_or_else(|| FALLBACK_USERNAME.to_owned())
}

/// `base` for the first attempt, then `base-2`, `base-3`, ... within the length limit.
pub fn username_candidate(base: &str, attempt: u32) -> String {
    if attempt == 0 {
        return base.to_owned();
    }
    let suffix = format!("-{}", attempt + 1);
    let keep = USERNAME_MAX_LENGTH
        .saturating_sub(suffix.len())
        .min(base.len());
    format!("{}{suffix}", &base[..keep])
}

fn is_separator(c: char) -> bool {
    matches!(c, '.' | '_' | '-')
}

/// Maps a claim to the username policy (ASCII letters, digits, `.`, `_`, `-`; 3 to 32
/// characters), or `None` if too little of it is left.
fn sanitize(raw: &str) -> Option<String> {
    let raw = raw.split_once('@').map_or(raw, |(local, _)| local);
    let mut out = String::new();
    for c in raw.chars() {
        let mapped = if c.is_whitespace() {
            "_"
        } else if c.is_ascii_alphanumeric() || is_separator(c) {
            out.push(c);
            continue;
        } else {
            transliterate(c)
        };
        for c in mapped.chars() {
            if !(is_separator(c) && out.ends_with(is_separator)) {
                out.push(c);
            }
        }
    }
    let trimmed = out.trim_matches(is_separator);
    let truncated =
        trimmed[..trimmed.len().min(USERNAME_MAX_LENGTH)].trim_end_matches(is_separator);
    (truncated.len() >= USERNAME_MIN_LENGTH).then(|| truncated.to_owned())
}

/// ASCII spelling of common Latin letters; everything else is dropped.
fn transliterate(c: char) -> &'static str {
    match c {
        'à' | 'á' | 'â' | 'ã' | 'å' | 'ā' | 'ą' => "a",
        'À' | 'Á' | 'Â' | 'Ã' | 'Å' | 'Ā' | 'Ą' => "A",
        'ä' | 'æ' => "ae",
        'Ä' | 'Æ' => "Ae",
        'ç' | 'ć' | 'č' => "c",
        'Ç' | 'Ć' | 'Č' => "C",
        'è' | 'é' | 'ê' | 'ë' | 'ē' | 'ę' | 'ě' => "e",
        'È' | 'É' | 'Ê' | 'Ë' | 'Ē' | 'Ę' | 'Ě' => "E",
        'ì' | 'í' | 'î' | 'ï' | 'ī' => "i",
        'Ì' | 'Í' | 'Î' | 'Ï' | 'Ī' => "I",
        'ñ' | 'ń' | 'ň' => "n",
        'Ñ' | 'Ń' | 'Ň' => "N",
        'ò' | 'ó' | 'ô' | 'õ' | 'ø' | 'ō' => "o",
        'Ò' | 'Ó' | 'Ô' | 'Õ' | 'Ø' | 'Ō' => "O",
        'ö' | 'œ' => "oe",
        'Ö' | 'Œ' => "Oe",
        'ù' | 'ú' | 'û' | 'ū' | 'ů' => "u",
        'Ù' | 'Ú' | 'Û' | 'Ū' | 'Ů' => "U",
        'ü' => "ue",
        'Ü' => "Ue",
        'ý' | 'ÿ' => "y",
        'Ý' => "Y",
        'ß' => "ss",
        'ł' => "l",
        'Ł' => "L",
        'ś' | 'š' => "s",
        'Ś' | 'Š' => "S",
        'ź' | 'ż' | 'ž' => "z",
        'Ź' | 'Ż' | 'Ž' => "Z",
        'ř' => "r",
        'Ř' => "R",
        'ť' => "t",
        'Ť' => "T",
        'ď' => "d",
        'Ď' => "D",
        _ => "",
    }
}

/// The groups in `claim`, which may be a dotted path such as `realm_access.roles`. A single
/// string counts as one group. `None` if the claim is missing.
pub fn groups(claims: &Claims, claim: &str) -> Option<Vec<String>> {
    let value = claims.get(claim).or_else(|| {
        let (first, rest) = claim.split_once('.')?;
        rest.split('.')
            .try_fold(claims.get(first)?, |value, key| value.get(key))
    })?;
    match value {
        Value::String(group) => Some(vec![group.clone()]),
        Value::Array(items) => Some(
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_owned))
                .collect(),
        ),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::auth::password::validate_username;

    fn claims(value: Value) -> Claims {
        let Value::Object(map) = value else {
            panic!("claims must be an object")
        };
        map
    }

    #[test]
    fn prefers_preferred_username_then_name_email_and_sub() {
        let all = json!({
            "sub": "f81d4fae",
            "preferred_username": "mara.l",
            "name": "Mara Lindqvist",
            "email": "mara@example.com",
        });
        assert_eq!(username_base(&claims(all)), "mara.l");
        assert_eq!(
            username_base(&claims(json!({ "sub": "x", "name": "Mara Lindqvist" }))),
            "Mara_Lindqvist"
        );
        assert_eq!(
            username_base(&claims(
                json!({ "sub": "x", "name": "李", "email": "mara@x.org" })
            )),
            "mara"
        );
        assert_eq!(
            username_base(&claims(
                json!({ "sub": "f81d4fae", "preferred_username": "ml" })
            )),
            "f81d4fae"
        );
    }

    #[test]
    fn falls_back_to_sub_and_a_generic_name() {
        assert_eq!(
            username_base(&claims(json!({ "sub": "248289761001", "email": "a@b.c" }))),
            "248289761001"
        );
        assert_eq!(username_base(&claims(json!({ "sub": "李" }))), "user");
        assert_eq!(username_base(&claims(json!({}))), "user");
    }

    #[test]
    fn sanitizes_to_the_username_policy() {
        for (raw, expected) in [
            ("mara@example.com", Some("mara")),
            ("Mära Øster", Some("Maera_Oster")),
            ("José  García", Some("Jose_Garcia")),
            ("__x--y__", Some("x--y")),
            ("a b", Some("a_b")),
            ("ab", None),
            ("李小龍", None),
            ("Straße", Some("Strasse")),
        ] {
            assert_eq!(sanitize(raw).as_deref(), expected, "{raw}");
        }
        let long = sanitize(&"x".repeat(40)).unwrap();
        assert_eq!(long.len(), USERNAME_MAX_LENGTH);
        let trailing = sanitize(&format!("{}-tail", "a".repeat(31))).unwrap();
        assert_eq!(trailing, "a".repeat(31));
    }

    #[test]
    fn candidates_stay_valid() {
        assert_eq!(username_candidate("mara", 0), "mara");
        assert_eq!(username_candidate("mara", 1), "mara-2");
        assert_eq!(username_candidate("mara", 41), "mara-42");
        let long = "x".repeat(USERNAME_MAX_LENGTH);
        for attempt in [0, 1, 9, 999] {
            let candidate = username_candidate(&long, attempt);
            assert_eq!(validate_username(&candidate), Ok(()), "{candidate}");
        }
        assert_eq!(
            username_candidate(&long, 9),
            format!("{}-10", "x".repeat(29))
        );
    }

    #[test]
    fn reads_groups() {
        let c = claims(json!({
            "groups": ["/admins", "users", 3],
            "role": "admin",
            "realm_access": { "roles": ["tube-admin"] },
            "weird": { "x": 1 },
        }));
        assert_eq!(
            groups(&c, "groups"),
            Some(vec!["/admins".into(), "users".into()])
        );
        assert_eq!(groups(&c, "role"), Some(vec!["admin".into()]));
        assert_eq!(
            groups(&c, "realm_access.roles"),
            Some(vec!["tube-admin".into()])
        );
        assert_eq!(groups(&c, "weird"), None);
        assert_eq!(groups(&c, "missing"), None);
        assert_eq!(groups(&c, "realm_access.missing"), None);
    }
}
