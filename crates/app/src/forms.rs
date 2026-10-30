//! HTML form detection and authentication flow.
//!
//! This module parses HTML to find forms and their input fields,
//! then generates ephemeral auth tokens for interactive authentication.

use anyhow::Result;
use regex::Regex;

/// A detected form field.
#[derive(Debug, Clone)]
pub struct FormField {
    /// The field's name attribute.
    pub name: String,
    /// The field's type (text, password, email, etc.).
    pub field_type: String,
    /// Whether the field is required.
    pub required: bool,
    /// Any placeholder text.
    pub placeholder: Option<String>,
}

/// A detected HTML form.
#[derive(Debug, Clone)]
pub struct DetectedForm {
    /// The form's action URL.
    pub action: String,
    /// The form's method (GET or POST).
    pub method: String,
    /// The fields in this form.
    pub fields: Vec<FormField>,
}

/// Parse HTML to find all forms and their input fields.
pub fn parse_forms(html: &str) -> Vec<DetectedForm> {
    let mut forms = Vec::new();

    // Find all <form> tags with their attributes
    let form_pattern = Regex::new(r"<form[^>]*>").unwrap();
    for cap in form_pattern.captures_iter(html) {
        let form_tag = &cap[0];

        // Extract action and method
        let action = extract_attribute(form_tag, "action").unwrap_or_else(|| "/".to_string());
        let method = extract_attribute(form_tag, "method")
            .map(|m| m.to_uppercase())
            .unwrap_or_else(|| "GET".to_string());

        // Find all input fields within this form
        // (simplified: find inputs that appear after this form tag)
        let form_start = cap.get(0).unwrap().start();
        let rest_of_html = &html[form_start..];

        // Find the end of this form (</form>)
        let form_end = rest_of_html.find("</form>").unwrap_or(rest_of_html.len());
        let form_content = &rest_of_html[..form_end];

        let fields = parse_form_fields(form_content);

        forms.push(DetectedForm {
            action,
            method,
            fields,
        });
    }

    forms
}

/// Parse input fields from form content.
fn parse_form_fields(html: &str) -> Vec<FormField> {
    let mut fields = Vec::new();

    // Find all <input> tags
    let input_pattern = Regex::new(r"<input[^>]*>").unwrap();
    for cap in input_pattern.captures_iter(html) {
        let input_tag = &cap[0];

        let name = match extract_attribute(input_tag, "name") {
            Some(n) => n,
            None => continue, // Skip inputs without names
        };

        let field_type = extract_attribute(input_tag, "type")
            .unwrap_or_else(|| "text".to_string());

        // Skip hidden fields
        if field_type == "hidden" {
            continue;
        }

        let required = input_tag.contains("required");
        let placeholder = extract_attribute(input_tag, "placeholder");

        fields.push(FormField {
            name,
            field_type,
            required,
            placeholder,
        });
    }

    // Also find <select> and <textarea> tags
    let select_pattern = Regex::new(r"<select[^>]*>").unwrap();
    for cap in select_pattern.captures_iter(html) {
        let select_tag = &cap[0];
        if let Some(name) = extract_attribute(select_tag, "name") {
            let required = select_tag.contains("required");
            fields.push(FormField {
                name,
                field_type: "select".to_string(),
                required,
                placeholder: None,
            });
        }
    }

    let textarea_pattern = Regex::new(r"<textarea[^>]*>").unwrap();
    for cap in textarea_pattern.captures_iter(html) {
        let textarea_tag = &cap[0];
        if let Some(name) = extract_attribute(textarea_tag, "name") {
            let required = textarea_tag.contains("required");
            fields.push(FormField {
                name,
                field_type: "textarea".to_string(),
                required,
                placeholder: None,
            });
        }
    }

    fields
}

/// Extract an attribute value from a tag.
fn extract_attribute(tag: &str, attr_name: &str) -> Option<String> {
    let lower = tag.to_lowercase();
    let search_term = format!("{}=", attr_name);
    if let Some(pos) = lower.find(&search_term) {
        let rest = &tag[pos + search_term.len()..];
        let rest = rest.trim_start();
        if let Some(quote_char) = rest.chars().next().filter(|&c| c == '"' || c == '\'') {
            if let Some(end_pos) = rest[1..].find(quote_char) {
                return Some(rest[1..end_pos + 1].to_string());
            }
        } else {
            // Unquoted value
            if let Some(end_pos) = rest.find(|c: char| c == ' ' || c == '>' || c == '/') {
                return Some(rest[..end_pos].to_string());
            }
            return Some(rest.to_string());
        }
    }
    None
}

/// Generate an ephemeral auth token for interactive authentication.
///
/// Returns a URL that the user can open in their browser to complete authentication.
pub async fn generate_auth_token(redirect_url: &str) -> Result<String> {
    // Use the auth crate's token generation
    let store = tuirowser_auth::TokenStore::new();

    let request = tuirowser_auth::TokenRequest {
        redirect_url: redirect_url.to_string(),
        ttl_secs: Some(300), // 5 minutes for interactive auth
        multi_use: false,
    };

    let token = store.generate(&request).await?;

    // Return the full URL with the token
    Ok(format!("http://127.0.0.1:8765/token/validate?token={}", token))
}

/// Check if a page contains any forms.
pub fn has_forms(html: &str) -> bool {
    html.contains("<form") || html.contains("<input")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_form() {
        let html = r#"
            <html>
                <body>
                    <form action="/login" method="post">
                        <input type="text" name="username" required>
                        <input type="password" name="password" required>
                        <input type="submit" value="Login">
                    </form>
                </body>
            </html>
        "#;

        let forms = parse_forms(html);
        assert_eq!(forms.len(), 1);

        let form = &forms[0];
        assert_eq!(form.action, "/login");
        assert_eq!(form.method, "POST");
        assert_eq!(form.fields.len(), 2); // username and password (submit is skipped)

        assert_eq!(form.fields[0].name, "username");
        assert_eq!(form.fields[0].field_type, "text");
        assert!(form.fields[0].required);

        assert_eq!(form.fields[1].name, "password");
        assert_eq!(form.fields[1].field_type, "password");
        assert!(form.fields[1].required);
    }

    #[test]
    fn test_parse_multiple_forms() {
        let html = r#"
            <html>
                <body>
                    <form action="/login" method="post">
                        <input type="text" name="username">
                    </form>
                    <form action="/search" method="get">
                        <input type="text" name="q">
                    </form>
                </body>
            </html>
        "#;

        let forms = parse_forms(html);
        assert_eq!(forms.len(), 2);
    }

    #[test]
    fn test_has_forms() {
        assert!(has_forms("<form action='/login'></form>"));
        assert!(has_forms("<input type='text' name='q'>"));
        assert!(!has_forms("<p>No forms here</p>"));
    }
}