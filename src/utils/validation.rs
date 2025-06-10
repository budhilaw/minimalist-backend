use once_cell::sync::Lazy;
use regex::Regex;

static EMAIL_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap());

static PHONE_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^\+?[1-9]\d{1,14}$").unwrap());

static SLUG_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[a-z0-9]+(?:-[a-z0-9]+)*$").unwrap());

pub fn is_valid_email(email: &str) -> bool {
    EMAIL_REGEX.is_match(email)
}

pub fn is_valid_phone(phone: &str) -> bool {
    PHONE_REGEX.is_match(phone)
}

pub fn is_valid_slug(slug: &str) -> bool {
    if slug.is_empty() || slug.len() > 255 {
        return false;
    }
    SLUG_REGEX.is_match(slug)
}

pub fn generate_slug(title: &str) -> String {
    title
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c
            } else if c.is_whitespace() || c == '_' {
                '-'
            } else {
                '\0' // Will be filtered out
            }
        })
        .filter(|&c| c != '\0')
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<&str>>()
        .join("-")
        .trim_matches('-')
        .to_string()
}

pub fn sanitize_html(input: &str) -> String {
    // Basic HTML sanitization - in production, consider using a proper HTML sanitizer
    input
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
        .replace('&', "&amp;")
}

pub fn is_valid_url(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://")
}

pub fn normalize_tags(tags: Vec<String>) -> Vec<String> {
    tags.into_iter()
        .map(|tag| tag.trim().to_lowercase())
        .filter(|tag| !tag.is_empty() && tag.len() <= 50)
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_validation() {
        assert!(is_valid_email("test@example.com"));
        assert!(is_valid_email("user.name+tag@domain.co.uk"));
        assert!(!is_valid_email("invalid-email"));
        assert!(!is_valid_email("@example.com"));
        assert!(!is_valid_email("test@"));
    }

    #[test]
    fn test_slug_generation() {
        assert_eq!(generate_slug("Hello World"), "hello-world");
        assert_eq!(generate_slug("My First Blog Post!"), "my-first-blog-post");
        assert_eq!(generate_slug("   Multiple   Spaces   "), "multiple-spaces");
        assert_eq!(
            generate_slug("Special-Characters@#$%"),
            "special-characters"
        );
    }

    #[test]
    fn test_slug_validation() {
        assert!(is_valid_slug("hello-world"));
        assert!(is_valid_slug("test123"));
        assert!(!is_valid_slug("Hello-World")); // uppercase
        assert!(!is_valid_slug("hello_world")); // underscore
        assert!(!is_valid_slug("-hello")); // starts with dash
        assert!(!is_valid_slug("hello-")); // ends with dash
    }
}
