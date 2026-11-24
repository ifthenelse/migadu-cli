//! The one seam between the program and the network: tests supply a fake `Http`.

use std::time::Duration;

pub struct Req {
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
}

pub struct Resp {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

impl Resp {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
}

pub trait Http {
    fn send(&self, req: Req) -> Result<Resp, String>;
}

/// Real HTTPS client (rustls). Redirects are not followed, so the admin panel's 302s stay visible.
pub struct Ureq {
    agent: ureq::Agent,
}

impl Ureq {
    pub fn new() -> Self {
        let config = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .max_redirects(0)
            .max_redirects_will_error(false)
            .timeout_global(Some(Duration::from_secs(30)))
            .build();
        Ureq {
            agent: config.into(),
        }
    }
}

impl Default for Ureq {
    fn default() -> Self {
        Self::new()
    }
}

impl Http for Ureq {
    fn send(&self, req: Req) -> Result<Resp, String> {
        let mut b = ureq::http::Request::builder()
            .method(req.method.as_str())
            .uri(&req.url);
        for (k, v) in &req.headers {
            b = b.header(k, v);
        }
        let result = match req.body {
            Some(body) => self.agent.run(b.body(body).map_err(|e| e.to_string())?),
            None => self.agent.run(b.body(()).map_err(|e| e.to_string())?),
        };
        let mut resp = result.map_err(|e| format!("{} {} failed: {e}", req.method, req.url))?;
        let headers = resp
            .headers()
            .iter()
            .map(|(k, v)| (k.as_str().to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();
        let status = resp.status().as_u16();
        let body = resp
            .body_mut()
            .read_to_string()
            .map_err(|e| e.to_string())?;
        Ok(Resp {
            status,
            headers,
            body,
        })
    }
}
