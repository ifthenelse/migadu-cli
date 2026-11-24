#![allow(dead_code)]
//! Test doubles: a scripted `Http` and `Exec`, and a runner that captures output.

use migadu::cli::{run, Io};
use migadu::http::{Http, Req, Resp};
use migadu::secrets::Exec;
use std::cell::RefCell;
use std::collections::HashMap;

pub struct Call {
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
}

impl Call {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
    pub fn json(&self) -> serde_json::Value {
        serde_json::from_str(self.body.as_deref().unwrap_or("null")).unwrap()
    }
    pub fn form(&self) -> Vec<(String, String)> {
        decode(self.body.as_deref().unwrap_or(""))
    }
}

pub type Responder<'a> = Box<dyn Fn(&Req) -> Resp + 'a>;

pub struct Fake<'a> {
    pub calls: RefCell<Vec<Call>>,
    responder: Responder<'a>,
}

impl<'a> Fake<'a> {
    pub fn new(responder: impl Fn(&Req) -> Resp + 'a) -> Self {
        Fake {
            calls: RefCell::new(vec![]),
            responder: Box::new(responder),
        }
    }
    pub fn urls(&self) -> Vec<String> {
        self.calls.borrow().iter().map(|c| c.url.clone()).collect()
    }
}

impl Http for Fake<'_> {
    fn send(&self, req: Req) -> Result<Resp, String> {
        let resp = (self.responder)(&req);
        self.calls.borrow_mut().push(Call {
            method: req.method,
            url: req.url,
            headers: req.headers,
            body: req.body,
        });
        Ok(resp)
    }
}

pub fn resp(status: u16, body: &str) -> Resp {
    Resp {
        status,
        headers: vec![],
        body: body.into(),
    }
}

pub fn redirect(location: &str) -> Resp {
    Resp {
        status: 302,
        headers: vec![("location".into(), location.into())],
        body: String::new(),
    }
}

pub fn json_resp(status: u16, v: serde_json::Value) -> Resp {
    resp(status, &v.to_string())
}

pub struct FakeExec(pub RefCell<Vec<Vec<String>>>);

impl FakeExec {
    pub fn new() -> Self {
        FakeExec(RefCell::new(vec![]))
    }
}

impl Exec for FakeExec {
    fn exec(&self, cmd: &str, args: &[String]) -> Result<String, String> {
        let mut c = vec![cmd.to_string()];
        c.extend(args.iter().cloned());
        self.0.borrow_mut().push(c);
        Ok("secret\nignored".into())
    }
}

pub struct Out {
    pub code: i32,
    pub out: Vec<String>,
    pub err: Vec<String>,
}

impl Out {
    pub fn json(&self) -> serde_json::Value {
        serde_json::from_str(&self.out.join("\n")).unwrap_or_else(|e| {
            panic!(
                "stdout is not JSON ({e}): {:?} / stderr {:?}",
                self.out, self.err
            )
        })
    }
    pub fn stderr(&self) -> String {
        self.err.join("\n")
    }
}

pub fn env(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

pub fn run_cli(
    argv: &[&str],
    env: &HashMap<String, String>,
    http: &dyn Http,
    exec: &dyn Exec,
) -> Out {
    let (mut out, mut err) = (vec![], vec![]);
    let mut o = |s: &str| out.push(s.to_string());
    let mut e = |s: &str| err.push(s.to_string());
    let stdin = || r#"{"a":1}"#.to_string();
    let argv: Vec<String> = argv.iter().map(|s| s.to_string()).collect();
    let code = {
        let mut io = Io {
            out: &mut o,
            err: &mut e,
            stdin: &stdin,
        };
        run(&argv, env, http, exec, &mut io)
    };
    Out { code, out, err }
}

pub fn decode(body: &str) -> Vec<(String, String)> {
    fn pct(s: &str) -> String {
        let b = s.as_bytes();
        let mut out = Vec::new();
        let mut i = 0;
        while i < b.len() {
            match b[i] {
                b'+' => out.push(b' '),
                b'%' => {
                    out.push(u8::from_str_radix(&s[i + 1..i + 3], 16).unwrap());
                    i += 2;
                }
                c => out.push(c),
            }
            i += 1;
        }
        String::from_utf8(out).unwrap()
    }
    body.split('&')
        .filter(|p| !p.is_empty())
        .map(|p| {
            let (k, v) = p.split_once('=').unwrap_or((p, ""));
            (pct(k), pct(v))
        })
        .collect()
}

pub fn get<'a>(form: &'a [(String, String)], k: &str) -> Option<&'a str> {
    form.iter().find(|(n, _)| n == k).map(|(_, v)| v.as_str())
}

pub fn all<'a>(form: &'a [(String, String)], k: &str) -> Vec<&'a str> {
    form.iter()
        .filter(|(n, _)| n == k)
        .map(|(_, v)| v.as_str())
        .collect()
}
