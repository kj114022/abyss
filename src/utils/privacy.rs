use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    // Detects common API key patterns (e.g. "api_key": "sk-...", "token": "...", "AWS_SECRET_ACCESS_KEY": "...")
    // Looking for key-value pairs where value looks like a secret
    static ref SECRET_PATTERNS: Vec<Regex> = vec![
        // Generic "key": "value" pattern for secrets
        Regex::new(r#"(?i)(api[_-]?key|password|secret|token|access[_-]?key|auth[_-]?token)[\"']?\s*[:=]\s*[\"'](?P<secret>[^\"']{8,})[\"']"#).unwrap(),
        // AWS Secret Key
        Regex::new(r#"(?i)aws_secret_access_key\s*=\s*(?P<secret>[A-Za-z0-9/+=]{40})"#).unwrap(),
        // Private Key Header
        Regex::new(r#"(?i)-{5}BEGIN (RSA|DSA|EC|OPENSSH) PRIVATE KEY-{5}"#).unwrap(),
    ];

    // Email Address
    static ref PII_EMAIL: Regex = Regex::new(r#"(?i)\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}\b"#).unwrap();

    // IPv4 Address (simple heuristic, avoids version numbers mostly)
    static ref PII_IPV4: Regex = Regex::new(r#"\b(?:[0-9]{1,3}\.){3}[0-9]{1,3}\b"#).unwrap();
}

/// Redacts sensitive information from the content based on predefined patterns.
pub fn redact_content(content: &str) -> String {
    let mut result = content.to_string();

    // Redact Secrets
    for pattern in SECRET_PATTERNS.iter() {
        if let Some(_caps) = pattern.captures(&result) {
            // Replacement uses closure to preserve context while redacting secret value
        }

        result = pattern
            .replace_all(&result, |caps: &regex::Captures| {
                // If "secret" group exists, preserve the full match but replace the secret part
                if let Some(m) = caps.name("secret") {
                    let full_match = caps.get(0).unwrap().as_str();
                    full_match.replace(m.as_str(), "[REDACTED]")
                } else {
                    // Replace whole match for patterns without secret group
                    "[REDACTED_SECRET]".to_string()
                }
            })
            .to_string();
    }

    // Redact PII
    result = PII_EMAIL
        .replace_all(&result, "[EMAIL_REDACTED]")
        .to_string();

    // IP Address - be careful not to redact version numbers 1.2.3
    // We skip for now to avoid noise, or use strict check.
    // result = PII_IPV4.replace_all(&result, "[IP_REDACTED]").to_string();

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redact_api_key() {
        let text = r#"const API_KEY = "sk-1234567890abcdef";"#;
        let redacted = redact_content(text);
        assert!(redacted.contains("[REDACTED]"));
        assert!(!redacted.contains("sk-1234567890abcdef"));
        assert!(redacted.contains("API_KEY"));
    }

    #[test]
    fn test_redact_json_token() {
        let text = r#"{"auth_token": "a1b2c3d4e5f6g7h8"}"#;
        let redacted = redact_content(text);
        assert!(redacted.contains(r#""auth_token": "[REDACTED]""#));
        assert!(!redacted.contains("a1b2c3d4e5f6g7h8"));
    }

    #[test]
    fn test_redact_email() {
        let text = "Contact support@example.com for help.";
        let redacted = redact_content(text);
        assert_eq!(redacted, "Contact [EMAIL_REDACTED] for help.");
    }
}
