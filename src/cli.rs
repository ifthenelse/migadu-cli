//! Argument parsing, command dispatch and output.

use crate::api::{Client, Result};
use crate::http::Http;
use crate::secrets::{load_credentials, resolve_secret, Exec};
use crate::web::{js_string, Web};
use serde_json::{json, Map, Value};
use std::collections::HashMap;

pub const HELP: &str = r#"migadu — Migadu email hosting from your terminal

Usage: migadu [options] <resource> <action> [args]

  tier                                     plan (needs MIGADU_WEB_COOKIE) and usage
  catchall get|set [domain]                catchall recipients (admin panel; needs MIGADU_WEB_COOKIE)
  spam get|set [domain]                    spam filtering (admin panel; needs MIGADU_WEB_COOKIE)
      keys: aggressiveness (3 permissive, 2 suspicious, 0 moderate, -2 aggressive, -3 paranoid),
      subject_rewriting, junk_subject_keyword, sender_denylist, sender_junklist, sender_allowlist,
      allowlist_precedence, recipient_denylist
  bcc list|create|update|delete            bcc captures (admin panel; needs MIGADU_WEB_COOKIE)
  domains  list|get|create|update|records|diagnostics|usage|activate [name]
  mailboxes|aliases|rewrites  list|get|create|update|delete [id]
  identities|forwardings      list|get|create|update|delete <mailbox> [id]
  api <METHOD> <path>                      raw call to any Migadu API endpoint

Options:
  -d, --domain <name>     domain (or MIGADU_DOMAIN)
  -f, --field k=v         body field (repeatable); k:=<json> for numbers/booleans/arrays
      --json <obj|@file|->  full JSON body
  -y, --yes               confirm delete
  -o, --output json|table output format (default json)
  -h, --help

Plan lookup reads the admin panel with a session cookie: MIGADU_WEB_COOKIE or MIGADU_WEB_COOKIE_REF.
Credentials: MIGADU_EMAIL plus MIGADU_API_KEY or MIGADU_API_KEY_REF
  (op://vault/item/field | bw:<item> | pass:<name> | keepassxc:<db>#<entry> | keychain:<service>)
"#;

/// Everything with a side effect the program needs besides the network and process spawning.
pub struct Io<'a> {
    pub out: &'a mut dyn FnMut(&str),
    pub err: &'a mut dyn FnMut(&str),
    pub stdin: &'a dyn Fn() -> String,
}

#[derive(Default)]
struct Opts {
    domain: Option<String>,
    field: Vec<String>,
    json: Option<String>,
    yes: bool,
    output: String,
    help: bool,
    pos: Vec<String>,
}

fn parse_args(args: &[String]) -> std::result::Result<Opts, String> {
    let mut o = Opts {
        output: "json".into(),
        ..Default::default()
    };
    let mut i = 0;
    let mut only_pos = false;
    let take = |i: &mut usize,
                label: &str,
                inline: Option<String>|
     -> std::result::Result<String, String> {
        if let Some(v) = inline {
            return Ok(v);
        }
        *i += 1;
        match args.get(*i) {
            Some(v) if v == "-" || !v.starts_with('-') => Ok(v.clone()),
            _ => Err(format!("Option '{label} <value>' argument missing")),
        }
    };
    while i < args.len() {
        let a = &args[i];
        if only_pos || a == "-" || !a.starts_with('-') {
            o.pos.push(a.clone());
        } else if a == "--" {
            only_pos = true;
        } else if let Some(long) = a.strip_prefix("--") {
            let (name, inline) = match long.split_once('=') {
                Some((n, v)) => (n, Some(v.to_string())),
                None => (long, None),
            };
            match name {
                "domain" => o.domain = Some(take(&mut i, "-d, --domain", inline)?),
                "field" => o.field.push(take(&mut i, "-f, --field", inline)?),
                "json" => o.json = Some(take(&mut i, "--json", inline)?),
                "output" => o.output = take(&mut i, "-o, --output", inline)?,
                "yes" | "help" if inline.is_none() => {
                    if name == "yes" {
                        o.yes = true
                    } else {
                        o.help = true
                    }
                }
                _ => return Err(format!("Unknown option '--{name}'")),
            }
        } else {
            let cluster: Vec<char> = a[1..].chars().collect();
            let mut j = 0;
            while j < cluster.len() {
                let c = cluster[j];
                let rest: String = cluster[j + 1..].iter().collect();
                let inline = if rest.is_empty() { None } else { Some(rest) };
                match c {
                    'y' => o.yes = true,
                    'h' => o.help = true,
                    'd' | 'f' | 'o' => {
                        let v = take(&mut i, &format!("-{c}"), inline)?;
                        match c {
                            'd' => o.domain = Some(v),
                            'f' => o.field.push(v),
                            _ => o.output = v,
                        }
                        break;
                    }
                    _ => return Err(format!("Unknown option '-{c}'")),
                }
                j += 1;
            }
        }
        i += 1;
    }
    Ok(o)
}

/// JavaScript's `encodeURIComponent`.
fn enc(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z'
            | b'a'..=b'z'
            | b'0'..=b'9'
            | b'-'
            | b'_'
            | b'.'
            | b'!'
            | b'~'
            | b'*'
            | b'\''
            | b'('
            | b')' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn build_body(o: &Opts, stdin: &dyn Fn() -> String) -> Result<Option<Value>> {
    let mut body = Value::Object(Map::new());
    if let Some(src) = &o.json {
        let text = if src == "-" {
            stdin()
        } else if let Some(path) = src.strip_prefix('@') {
            std::fs::read_to_string(path).map_err(|e| format!("cannot read {path}: {e}"))?
        } else {
            src.clone()
        };
        body = serde_json::from_str(&text).map_err(|e| format!("--json is not valid JSON: {e}"))?;
    }
    for f in &o.field {
        let i = f
            .find('=')
            .filter(|i| *i >= 1)
            .ok_or_else(|| format!("bad --field \"{f}\", expected k=v or k:=json"))?;
        let typed = f.as_bytes()[i - 1] == b':';
        let k = &f[..if typed { i - 1 } else { i }];
        let v = &f[i + 1..];
        let value = if typed {
            serde_json::from_str(v).map_err(|e| format!("bad --field \"{f}\": {e}"))?
        } else {
            json!(v)
        };
        match &mut body {
            Value::Object(m) => {
                m.insert(k.to_string(), value);
            }
            _ => return Err("--json must be an object when used with --field".into()),
        }
    }
    let empty = match &body {
        Value::Object(m) => m.is_empty(),
        Value::Array(a) => a.is_empty(),
        _ => false,
    };
    Ok(if empty { None } else { Some(body) })
}

fn table(data: &Value) -> String {
    let pretty = || serde_json::to_string_pretty(data).unwrap_or_default();
    let rows: &[Value] = match data {
        Value::Array(a) => a,
        Value::Object(m) => match m.values().find_map(|v| v.as_array()) {
            Some(a) => a,
            None => return pretty(),
        },
        _ => return pretty(),
    };
    let Some(Value::Object(first)) = rows.first() else {
        return pretty();
    };
    let cols: Vec<&String> = first
        .iter()
        .filter(|(_, v)| !matches!(v, Value::Object(_) | Value::Array(_) | Value::Null))
        .map(|(k, _)| k)
        .take(6)
        .collect();
    let cell = |r: &Value, c: &str| match r.get(c) {
        None | Some(Value::Null) => String::new(),
        Some(v) => js_string(v),
    };
    let widths: Vec<usize> = cols
        .iter()
        .map(|c| {
            rows.iter()
                .map(|r| cell(r, c).chars().count())
                .chain([c.chars().count()])
                .max()
                .unwrap_or(0)
        })
        .collect();
    let line = |get: &dyn Fn(&str) -> String| {
        cols.iter()
            .zip(&widths)
            .map(|(c, w)| format!("{:<w$}", get(c), w = *w))
            .collect::<Vec<_>>()
            .join("  ")
    };
    let mut lines = vec![line(&|c: &str| c.to_string())];
    for r in rows {
        lines.push(line(&|c: &str| cell(r, c)));
    }
    lines.join("\n")
}

/// Plan names are not documented in the API; surface any plan/tier-looking fields.
fn find_plan_fields(v: &Value, path: &str, out: &mut Map<String, Value>) {
    let entries: Vec<(String, &Value)> = match v {
        Value::Object(m) => m.iter().map(|(k, v)| (k.clone(), v)).collect(),
        Value::Array(a) => a
            .iter()
            .enumerate()
            .map(|(i, v)| (i.to_string(), v))
            .collect(),
        _ => return,
    };
    for (k, v) in entries {
        if matches!(v, Value::Object(_) | Value::Array(_)) {
            find_plan_fields(v, &format!("{path}{k}."), out);
        } else {
            let lk = k.to_lowercase();
            if lk.contains("plan") || lk.contains("tier") {
                out.insert(format!("{path}{k}"), v.clone());
            }
        }
    }
}

fn tier(client: Option<&Client>, domain: Option<&str>, web: Option<&Web>) -> Result<Value> {
    let mut report = Map::new();
    report.insert("plan_fields".into(), json!({}));
    report.insert("domains".into(), json!([]));
    report.insert("note".into(), json!(""));
    if let Some(w) = web {
        report.extend(w.plan()?);
    }
    let Some(client) = client else {
        return Ok(Value::Object(report));
    };
    let list = client.request("GET", "/domains", None)?;
    let mut fields = Map::new();
    let mut domains = Vec::new();
    for d in list
        .get("domains")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
    {
        let name = d
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("undefined")
            .to_string();
        if domain.is_some_and(|x| x != name) {
            continue;
        }
        let usage = client
            .request("GET", &format!("/domains/{}/usage", enc(&name)), None)
            .unwrap_or_else(|e| json!({ "error": e }));
        find_plan_fields(&d, &format!("{name}."), &mut fields);
        find_plan_fields(&usage, &format!("{name}.usage."), &mut fields);
        let mut entry = Map::new();
        entry.insert("name".into(), json!(name));
        if let Some(a) = d.get("active") {
            entry.insert("active".into(), a.clone());
        }
        entry.insert("usage".into(), usage);
        domains.push(Value::Object(entry));
    }
    if web.is_none() && fields.is_empty() {
        report.insert(
            "note".into(),
            json!("The Migadu API returns no plan/tier field. Set MIGADU_WEB_COOKIE (or MIGADU_WEB_COOKIE_REF) to read the plan from the admin panel."),
        );
    }
    report.insert("plan_fields".into(), Value::Object(fields));
    report.insert("domains".into(), Value::Array(domains));
    Ok(Value::Object(report))
}

const DOMAIN_ACTIONS: &[(&str, &str)] = &[
    ("records", "GET"),
    ("diagnostics", "GET"),
    ("usage", "GET"),
    ("activate", "POST"),
];

/// Builds a resource's collection path from (domain, mailbox).
type PathFn = fn(&str, &str) -> String;

/// (resource, path builder, needs a mailbox first)
fn resource(res: &str) -> Option<(PathFn, bool)> {
    Some(match res {
        "mailboxes" => (|d, _| format!("/domains/{}/mailboxes", enc(d)), false),
        "aliases" => (|d, _| format!("/domains/{}/aliases", enc(d)), false),
        "rewrites" => (|d, _| format!("/domains/{}/rewrites", enc(d)), false),
        "identities" => (
            |d, m| format!("/domains/{}/mailboxes/{}/identities", enc(d), enc(m)),
            true,
        ),
        "forwardings" => (
            |d, m| format!("/domains/{}/mailboxes/{}/forwardings", enc(d), enc(m)),
            true,
        ),
        _ => return None,
    })
}

fn nonempty(env: &HashMap<String, String>, k: &str) -> Option<String> {
    env.get(k).filter(|s| !s.is_empty()).cloned()
}

/// Run the CLI. Returns the process exit code.
pub fn run(
    argv: &[String],
    env: &HashMap<String, String>,
    http: &dyn Http,
    exec: &dyn Exec,
    io: &mut Io,
) -> i32 {
    let o = match parse_args(argv) {
        Ok(o) => o,
        Err(e) => {
            (io.err)(&e);
            return 2;
        }
    };
    let res = o.pos.first().cloned();
    if o.help || res.is_none() {
        (io.out)(HELP.trim_end_matches('\n'));
        return if res.is_some() || o.help { 0 } else { 2 };
    }
    match dispatch(&o, env, http, exec, io.stdin) {
        Ok(result) => {
            let text = if o.output == "table" {
                table(&result)
            } else {
                serde_json::to_string_pretty(&result).unwrap_or_default()
            };
            (io.out)(&text);
            0
        }
        Err(e) => {
            (io.err)(&format!("error: {e}"));
            1
        }
    }
}

fn dispatch(
    o: &Opts,
    env: &HashMap<String, String>,
    http: &dyn Http,
    exec: &dyn Exec,
    stdin: &dyn Fn() -> String,
) -> Result<Value> {
    let res = o.pos[0].as_str();
    let action = o.pos.get(1).map(String::as_str);
    let mut rest: Vec<String> = o.pos.iter().skip(2).cloned().collect();
    let is_web = matches!(res, "tier" | "catchall" | "spam" | "bcc");

    let cookie = if is_web {
        match nonempty(env, "MIGADU_WEB_COOKIE") {
            Some(c) => Some(c),
            None => match nonempty(env, "MIGADU_WEB_COOKIE_REF") {
                Some(r) => Some(resolve_secret(&r, exec, std::env::consts::OS)?),
                None => None,
            },
        }
    } else {
        None
    };
    let web = cookie
        .as_deref()
        .filter(|c| !c.is_empty())
        .map(|c| Web::new(c, http));
    // `tier` may run on the web cookie alone; everything else needs API credentials.
    let creds = match load_credentials(env, exec) {
        Ok(c) => Some(c),
        Err(e) if web.is_some() => {
            let _ = e;
            None
        }
        Err(e) => return Err(e),
    };
    let client = creds
        .as_ref()
        .map(|c| Client::new(&c.email, &c.api_key, http));
    let domain = o
        .domain
        .clone()
        .or_else(|| env.get("MIGADU_DOMAIN").cloned());
    let need_domain = || {
        domain
            .clone()
            .ok_or_else(|| "domain required: use --domain or MIGADU_DOMAIN".to_string())
    };
    let api = || {
        client
            .as_ref()
            .ok_or_else(|| "API credentials are required".to_string())
    };
    let body = build_body(o, stdin)?;
    let body_map = || {
        body.as_ref()
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default()
    };

    match res {
        "tier" => tier(client.as_ref(), domain.as_deref(), web.as_ref()),
        "catchall" => {
            let web = web.as_ref().ok_or(
                "catchall uses the admin panel: set MIGADU_WEB_COOKIE or MIGADU_WEB_COOKIE_REF",
            )?;
            let name = rest.first().cloned().map_or_else(need_domain, Ok)?;
            match action {
                Some("get") => web.get_catchall(&name),
                Some("set") => {
                    let raw = body_map()
                        .get("destinations")
                        .map(js_string)
                        .unwrap_or_default();
                    let list: Vec<String> = raw
                        .split(|c: char| c.is_whitespace() || c == ',')
                        .filter(|s| !s.is_empty())
                        .map(String::from)
                        .collect();
                    if list.is_empty() && !o.yes {
                        return Err("refusing to clear the catchall without --yes".into());
                    }
                    web.set_catchall(&name, &list)
                }
                _ => Err(
                    "usage: migadu catchall get|set [domain] -f destinations=a@x.org,b@x.org"
                        .into(),
                ),
            }
        }
        "spam" => {
            let web = web.as_ref().ok_or(
                "spam uses the admin panel: set MIGADU_WEB_COOKIE or MIGADU_WEB_COOKIE_REF",
            )?;
            let name = rest.first().cloned().map_or_else(need_domain, Ok)?;
            match action {
                Some("get") => Ok(Value::Object(web.get_spam(&name)?)),
                Some("set") => {
                    let changes = body_map();
                    if changes.is_empty() {
                        return Err(
                            "nothing to set: use -f aggressiveness=0 -f subject_rewriting:=true …"
                                .into(),
                        );
                    }
                    web.set_spam(&name, &changes)
                }
                _ => Err("usage: migadu spam get|set [domain] -f key=value".into()),
            }
        }
        "bcc" => {
            let web = web.as_ref().ok_or(
                "bcc uses the admin panel: set MIGADU_WEB_COOKIE or MIGADU_WEB_COOKIE_REF",
            )?;
            let from_opts = o
                .domain
                .clone()
                .or_else(|| env.get("MIGADU_DOMAIN").cloned());
            let name = from_opts
                .clone()
                .or_else(|| rest.first().cloned())
                .ok_or("domain required: use --domain or MIGADU_DOMAIN")?;
            let arg = if from_opts.as_deref().is_some_and(|s| !s.is_empty()) {
                rest.first()
            } else {
                rest.get(1)
            }
            .cloned();
            match (action, arg) {
                (Some("list"), _) => Ok(Value::Array(web.list_bcc(&name)?.into_iter().map(Value::Object).collect())),
                (Some("create"), _) => Ok(Value::Object(web.create_bcc(&name, &body_map())?)),
                (Some("update"), Some(a)) => web.update_bcc(&name, &a, &body_map()),
                (Some("delete"), Some(a)) => {
                    if !o.yes {
                        return Err(format!("refusing to delete bcc capture \"{a}\" without --yes"));
                    }
                    web.delete_bcc(&name, &a)
                }
                _ => Err("usage: migadu bcc list|create|update <address>|delete <address> -d <domain> [-f address= -f destination= -f direction=]".into()),
            }
        }
        "api" => {
            let (Some(method), Some(path)) = (action, rest.first()) else {
                return Err("usage: migadu api <METHOD> <path>".into());
            };
            let path = if path.starts_with('/') {
                path.clone()
            } else {
                format!("/{path}")
            };
            api()?.request(&method.to_uppercase(), &path, body.as_ref())
        }
        "domains" => {
            let client = api()?;
            let name = rest.first().cloned();
            let action = action.unwrap_or("undefined");
            match (action, name) {
                ("list", _) => client.request("GET", "/domains", None),
                ("create", _) => client.request("POST", "/domains", body.as_ref()),
                (_, None) => Err(format!("usage: migadu domains {action} <name>")),
                ("get", Some(n)) => client.request("GET", &format!("/domains/{}", enc(&n)), None),
                ("update", Some(n)) => {
                    client.request("PUT", &format!("/domains/{}", enc(&n)), body.as_ref())
                }
                (a, Some(n)) => match DOMAIN_ACTIONS.iter().find(|(k, _)| *k == a) {
                    Some((_, method)) => {
                        client.request(method, &format!("/domains/{}/{a}", enc(&n)), body.as_ref())
                    }
                    None => Err(format!("unknown domains action \"{a}\"")),
                },
            }
        }
        _ => {
            let Some((path, scoped)) = resource(res) else {
                return Err(format!("unknown resource \"{res}\""));
            };
            let client = api()?;
            let action = action.unwrap_or("undefined");
            let mailbox = if scoped {
                if rest.is_empty() {
                    return Err(format!("usage: migadu {res} {action} <mailbox> [id]"));
                }
                rest.remove(0)
            } else {
                String::new()
            };
            let base = path(&need_domain()?, &mailbox);
            let id = rest.first().cloned();
            let item = || {
                id.as_ref()
                    .map(|i| format!("{base}/{}", enc(i)))
                    .ok_or_else(|| {
                        format!(
                            "usage: migadu {res} {action} {}<id>",
                            if scoped { "<mailbox> " } else { "" }
                        )
                    })
            };
            match action {
                "list" => client.request("GET", &base, None),
                "get" => client.request("GET", &item()?, None),
                "create" => client.request("POST", &base, body.as_ref()),
                "update" => client.request("PUT", &item()?, body.as_ref()),
                "delete" => {
                    if !o.yes {
                        return Err(format!(
                            "refusing to delete {res} \"{}\" without --yes",
                            id.as_deref().unwrap_or("undefined")
                        ));
                    }
                    client.request("DELETE", &item()?, None)
                }
                _ => Err(format!("unknown {res} action \"{action}\"")),
            }
        }
    }
}
