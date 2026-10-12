//! Best-effort secret scrubbing for outbound decision state.
//!
//! **This is data minimization, not anonymization.** [`redact`] removes
//! common credential shapes — `sk-…` bearer-style tokens, `password=`/
//! `token=`/`secret=`/`api_key=` style pairs, JSON `"password": "…"`
//! pairs, and long hex/base64 blobs — before the state is sent to a JEV
//! endpoint. It cannot guarantee that every sensitive value is caught;
//! callers should still avoid putting secrets into state in the first
//! place, and should review what they send.

use serde_json::Value;

/// Replacement text substituted for redacted values.
pub const MASK: &str = "[REDACTED]";

/// Key-name suffixes (lowercase) that mark a `key=value` / `"key": "value"`
/// pair as containing a secret. Suffix matching means `db_password` and
/// `X-Api-Key` are caught too.
const SECRET_KEY_SUFFIXES: &[&str] = &[
    "password",
    "passwd",
    "pwd",
    "secret",
    "token",
    "api_key",
    "apikey",
    "api-key",
    "access_key",
    "access-token",
    "access_token",
    "auth",
    "authorization",
    "credential",
    "private_key",
    "bearer",
];

/// Minimum length of a `sk-…` token body before it is treated as a secret.
const SK_MIN_BODY: usize = 4;
/// Minimum length for a base64-looking blob to be masked.
const BLOB_MIN_LEN: usize = 40;
/// Minimum length for a hex-looking blob (hashes, keys) to be masked.
const HEX_MIN_LEN: usize = 32;

/// Scrub obvious secrets from `input`.
///
/// Best-effort only — see the module docs. Idempotent: running the result
/// through `redact` again produces the same output.
pub fn redact(input: &str) -> String {
    if input.is_empty() {
        return String::new();
    }
    // Prefix pass first so `Authorization: Bearer <token>` loses the whole
    // token, not just the word after the colon.
    let step1 = redact_prefixed_tokens(input);
    let step2 = redact_pairs(&step1);
    redact_long_blobs(&step2)
}

/// Recursively redact every string leaf of a JSON value.
///
/// Returns the scrubbed value and whether anything changed.
pub fn redact_value(value: &Value) -> (Value, bool) {
    match value {
        Value::String(s) => {
            let scrubbed = redact(s);
            let changed = scrubbed != *s;
            (Value::String(scrubbed), changed)
        }
        Value::Array(items) => {
            let mut changed = false;
            let items = items
                .iter()
                .map(|v| {
                    let (v, c) = redact_value(v);
                    changed |= c;
                    v
                })
                .collect();
            (Value::Array(items), changed)
        }
        Value::Object(map) => {
            let mut changed = false;
            let out = map
                .iter()
                .map(|(k, v)| {
                    let (v, c) = redact_value(v);
                    changed |= c;
                    (k.clone(), v)
                })
                .collect();
            (Value::Object(out), changed)
        }
        other => (other.clone(), false),
    }
}

fn is_key_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '-'
}

fn key_is_secret(key_lower: &str) -> bool {
    SECRET_KEY_SUFFIXES
        .iter()
        .any(|suffix| key_lower.ends_with(suffix))
}

fn is_value_end(c: char) -> bool {
    c.is_whitespace() || matches!(c, '"' | '\'' | '&' | ';' | ',' | ')' | ']' | '}' | '>' | '[')
}

/// Redact `password=…`, `token: …`, and JSON `"password": "…"` pairs.
fn redact_pairs(input: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
    let mut out = String::with_capacity(input.len());
    let mut last = 0; // next char index still to be copied
    let mut i = 0;
    while i < chars.len() {
        let sep = chars[i];
        if sep != '=' && sep != ':' {
            i += 1;
            continue;
        }
        // JSON keys are quoted (`"password": "…"`), so step over one closing
        // quote before scanning the bare key run.
        let quoted = i > 0 && chars[i - 1] == '"';
        let mut k = i;
        if quoted {
            k -= 1;
        }
        while k > 0 && is_key_char(chars[k - 1]) {
            k -= 1;
        }
        // A quoted key must also have its opening quote, otherwise this is
        // not a JSON pair and we leave it alone.
        if quoted {
            if k == 0 || chars[k - 1] != '"' {
                i += 1;
                continue;
            }
            k -= 1;
        }
        if k == i {
            i += 1;
            continue;
        }
        // Slice the key out, excluding both quotes for the JSON form.
        let key_start = if quoted { k + 1 } else { k };
        let key_end = if quoted { i - 1 } else { i };
        let key: String = chars[key_start..key_end]
            .iter()
            .collect::<String>()
            .to_lowercase();
        if !key_is_secret(&key) {
            i += 1;
            continue;
        }
        // Skip spaces after the separator.
        let mut v = i + 1;
        while v < chars.len() && (chars[v] == ' ' || chars[v] == '\t') {
            v += 1;
        }
        if v >= chars.len() {
            break;
        }
        if chars[v] == '"' {
            // Quoted value (JSON form): mask the string contents, keep the
            // quotes so surrounding JSON stays valid.
            let mut end = v + 1;
            while end < chars.len() {
                if chars[end] == '\\' && end + 1 < chars.len() {
                    end += 2;
                    continue;
                }
                if chars[end] == '"' {
                    break;
                }
                end += 1;
            }
            if end >= chars.len() {
                i += 1;
                continue; // unterminated string — leave untouched
            }
            out.extend(&chars[last..v + 1]);
            out.push_str(MASK);
            out.extend(&chars[end..end + 1]);
            last = end + 1;
            i = end + 1;
        } else {
            // Bare value: consume until a delimiter.
            let mut end = v;
            while end < chars.len() && !is_value_end(chars[end]) {
                end += 1;
            }
            if end == v {
                i += 1;
                continue; // empty value — nothing to redact
            }
            out.extend(&chars[last..v]);
            out.push_str(MASK);
            last = end;
            i = end;
        }
    }
    out.extend(&chars[last..]);
    out
}

fn is_token_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '+' | '/' | '=')
}

/// Redact `sk-…` tokens and `Bearer <token>` credentials anywhere in text.
fn redact_prefixed_tokens(input: &str) -> String {
    // ASCII lowercase is length-preserving, keeping char indices in sync.
    let lower: Vec<char> = input.to_ascii_lowercase().chars().collect();
    let orig: Vec<char> = input.chars().collect();
    let mut out = String::with_capacity(input.len());
    let mut last = 0;
    let mut i = 0;
    while i < orig.len() {
        let at_boundary = i == 0 || !is_token_char(orig[i - 1]);
        // `sk-` prefix (case-insensitive).
        let is_sk = at_boundary
            && i + 2 < orig.len()
            && lower[i] == 's'
            && lower[i + 1] == 'k'
            && orig[i + 2] == '-';
        // `Bearer ` prefix (case-insensitive).
        let is_bearer = at_boundary
            && i + 6 < orig.len()
            && lower[i] == 'b'
            && lower[i + 1] == 'e'
            && lower[i + 2] == 'a'
            && lower[i + 3] == 'r'
            && lower[i + 4] == 'e'
            && lower[i + 5] == 'r'
            && orig[i + 6] == ' ';
        let (token_start, min_body) = if is_sk {
            (i + 3, SK_MIN_BODY)
        } else if is_bearer {
            (i + 7, 8)
        } else {
            i += 1;
            continue;
        };
        let mut end = token_start;
        while end < orig.len() && is_token_char(orig[end]) {
            end += 1;
        }
        if end - token_start < min_body {
            i += 1;
            continue;
        }
        out.push_str(&chars_take(&orig[last..i]));
        out.push_str(MASK);
        last = end;
        i = end;
    }
    out.push_str(&chars_take(&orig[last..]));
    out
}

/// Join a char slice into a String (helper for clarity at call sites).
fn chars_take(chars: &[char]) -> String {
    chars.iter().collect()
}

fn is_blob_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '+' | '/' | '=' | '-' | '_')
}

/// Redact long hex and base64-looking blobs.
fn redact_long_blobs(input: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
    let mut out = String::with_capacity(input.len());
    let mut last = 0;
    let mut i = 0;
    while i < chars.len() {
        if !is_blob_char(chars[i]) {
            i += 1;
            continue;
        }
        let mut end = i;
        while end < chars.len() && is_blob_char(chars[end]) {
            end += 1;
        }
        let len = end - i;
        let run: String = chars[i..end].iter().collect();
        let is_hex = run.chars().all(|c| c.is_ascii_hexdigit());
        let has_digit = run.chars().any(|c| c.is_ascii_digit());
        let has_b64_symbol = run.contains(['+', '/', '=']);
        let mask = (len >= BLOB_MIN_LEN && (has_digit || has_b64_symbol))
            || (len >= HEX_MIN_LEN && is_hex);
        if mask {
            out.push_str(&chars_take(&chars[last..i]));
            out.push_str(MASK);
            last = end;
        }
        i = end;
    }
    out.push_str(&chars_take(&chars[last..]));
    out
}
