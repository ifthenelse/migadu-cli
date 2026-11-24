//! REST client for https://api.migadu.com/v1 (HTTP Basic auth: account email + API key).

use crate::http::{Http, Req};
use serde_json::{json, Value};

pub const BASE_URL: &str = "https://api.migadu.com/v1";

pub type Result<T> = std::result::Result<T, String>;

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub fn base64(input: &[u8]) -> String {
    let mut out = String::new();
    for chunk in input.chunks(3) {
        let n = (chunk[0] as u32) << 16
            | (*chunk.get(1).unwrap_or(&0) as u32) << 8
            | *chunk.get(2).unwrap_or(&0) as u32;
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(B64[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

pub struct Client<'a> {
    auth: String,
    base: String,
    http: &'a dyn Http,
}

impl<'a> Client<'a> {
    pub fn new(email: &str, api_key: &str, http: &'a dyn Http) -> Self {
        Self::with_base(email, api_key, http, BASE_URL)
    }

    pub fn with_base(email: &str, api_key: &str, http: &'a dyn Http, base: &str) -> Self {
        Client {
            auth: format!("Basic {}", base64(format!("{email}:{api_key}").as_bytes())),
            base: base.into(),
            http,
        }
    }

    fn raw(&self, method: &str, path: &str, body: Option<&Value>) -> Result<(u16, Value)> {
        let mut headers = vec![
            ("Authorization".to_string(), self.auth.clone()),
            ("Accept".into(), "application/json".into()),
        ];
        if body.is_some() {
            headers.push(("Content-Type".into(), "application/json".into()));
        }
        let resp = self.http.send(Req {
            method: method.into(),
            url: format!("{}{}", self.base, path),
            headers,
            body: body.map(|b| b.to_string()),
        })?;
        let data = if resp.body.is_empty() {
            Value::Null
        } else {
            serde_json::from_str(&resp.body).unwrap_or_else(|_| json!({ "raw": resp.body }))
        };
        Ok((resp.status, data))
    }

    /// Send a request; HTTP errors become `METHOD path -> HTTP status: message`.
    pub fn request(&self, method: &str, path: &str, body: Option<&Value>) -> Result<Value> {
        let (status, data) = self.raw(method, path, body)?;
        if (200..300).contains(&status) {
            return Ok(data);
        }
        // Migadu sometimes answers a successful DELETE with HTTP 500: confirm by re-reading.
        if method == "DELETE" && status == 500 && self.raw("GET", path, None)?.0 == 404 {
            return Ok(json!({ "deleted": true }));
        }
        let detail = match data.get("error") {
            Some(Value::String(s)) if !s.is_empty() => format!(": {s}"),
            Some(Value::Null) | Some(Value::Bool(false)) | Some(Value::String(_)) | None => {
                String::new()
            }
            Some(other) => format!(": {other}"),
        };
        Err(format!("{method} {path} -> HTTP {status}{detail}"))
    }
}

#[cfg(test)]
mod tests {
    use super::base64;

    #[test]
    fn base64_matches_known_vectors() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"a@x.org:k"), "YUB4Lm9yZzpr");
    }
}
