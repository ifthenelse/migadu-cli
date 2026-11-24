//! Admin-panel access (server-rendered HTML) with a session cookie the user supplies.
//!
//! shortcut: the HTML is read with the small scanner in `html.rs`; it breaks if Migadu changes
//! the markup. Every write is re-read and fails loudly if the panel did not apply it.

use crate::api::Result;
use crate::html::{
    self, attr, elements, parse_form, set_field, start_tags, strip_comments, FieldValue, Form,
};
use crate::http::{Http, Req};
use serde_json::{json, Map, Value};

pub const PANEL: &str = "https://admin.migadu.com";

/// Snapshot of the public plan table (admin.migadu.com/billing/subscription, 2026-11-24).
fn plan_limits(plan: &str) -> Option<Value> {
    let (in_d, out_d, gb, api) = match plan {
        "maxi" => (10000, 2000, 500, true),
        "standard" => (3000, 500, 100, true),
        "mini" => (1000, 100, 30, true),
        "micro" => (200, 20, 5, false),
        _ => return None,
    };
    Some(json!({ "in_per_day": in_d, "out_per_day": out_d, "storage_gb": gb, "api": api }))
}

/// JavaScript-style `String(value)` for values typed on the command line.
pub fn js_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => match n.as_f64() {
            Some(f) if n.is_f64() && f.fract() == 0.0 && f.abs() < 1e15 => format!("{}", f as i64),
            _ => n.to_string(),
        },
        Value::Bool(b) => b.to_string(),
        Value::Null => "null".into(),
        Value::Array(a) => a.iter().map(js_string).collect::<Vec<_>>().join(","),
        Value::Object(_) => "[object Object]".into(),
    }
}

fn truthy(v: &Value) -> bool {
    matches!(v, Value::Bool(true)) || matches!(v, Value::String(s) if s == "true")
}

fn tokens(v: &str) -> Vec<String> {
    v.split(|c: char| c.is_whitespace() || c == ',')
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect()
}

/// application/x-www-form-urlencoded, as a browser (and `URLSearchParams`) writes it.
pub fn form_urlencode(fields: &[(String, String)]) -> String {
    fn enc(s: &str) -> String {
        let mut out = String::new();
        for b in s.bytes() {
            match b {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'*' | b'-' | b'.' | b'_' => {
                    out.push(b as char)
                }
                b' ' => out.push('+'),
                _ => out.push_str(&format!("%{b:02X}")),
            }
        }
        out
    }
    fields
        .iter()
        .map(|(k, v)| format!("{}={}", enc(k), enc(v)))
        .collect::<Vec<_>>()
        .join("&")
}

fn field_value(v: Option<FieldValue>) -> Option<Value> {
    v.map(|v| match v {
        FieldValue::Bool(b) => Value::Bool(b),
        FieldValue::Text(t) => Value::String(t),
    })
}

fn local_part(a: &str) -> &str {
    a.split('@').next().unwrap_or("")
}

// ----- tiny scanners for the few places that are not forms --------------------------------------

/// `href="/billing/subscription" ...>Mini / monthly<` -> ("mini", "monthly")
fn parse_plan_header(h: &str) -> Option<(String, String)> {
    const NEEDLE: &str = "href=\"/billing/subscription\"";
    let mut from = 0;
    while let Some(p) = h[from..].find(NEEDLE) {
        let after = from + p + NEEDLE.len();
        from = after;
        let Some(gt) = h[after..].find('>') else {
            continue;
        };
        let rest = h[after + gt + 1..].trim_start();
        let word = |s: &str| -> usize { s.bytes().take_while(|b| b.is_ascii_alphabetic()).count() };
        let n1 = word(rest);
        if n1 == 0 {
            continue;
        }
        let Some(rest2) = rest[n1..].trim_start().strip_prefix('/') else {
            continue;
        };
        let rest2 = rest2.trim_start();
        let n2 = word(rest2);
        if n2 == 0 || !rest2[n2..].trim_start().starts_with('<') {
            continue;
        }
        return Some((rest[..n1].to_lowercase(), rest2[..n2].to_lowercase()));
    }
    None
}

/// `renews November 27, 2026`
fn parse_renews(h: &str) -> Option<String> {
    let mut from = 0;
    while let Some(p) = h[from..].find("renews") {
        let after = from + p + 6;
        from = after;
        let rest = &h[after..];
        let trimmed = rest.trim_start();
        if trimmed.len() == rest.len() {
            continue;
        }
        let b = trimmed.as_bytes();
        if b.first().is_none_or(|c| !c.is_ascii_uppercase()) {
            continue;
        }
        let mut i = 1;
        while i < b.len() && b[i].is_ascii_lowercase() {
            i += 1;
        }
        if i == 1 || b.get(i) != Some(&b' ') {
            continue;
        }
        i += 1;
        let d = b[i..]
            .iter()
            .take_while(|c| c.is_ascii_digit())
            .count()
            .min(2);
        if d == 0 || !trimmed[i + d..].starts_with(", ") {
            continue;
        }
        let y = i + d + 2;
        if b.len() >= y + 4 && b[y..y + 4].iter().all(|c| c.is_ascii_digit()) {
            return Some(trimmed[..y + 4].to_string());
        }
    }
    None
}

/// `href="/domains/123">example.org<` -> (id, name)
fn parse_domains(h: &str) -> Vec<(String, String)> {
    let lower = h.to_ascii_lowercase();
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(p) = lower[from..].find("href=\"/domains/") {
        let at = from + p + 15;
        from = at;
        let digits = lower[at..]
            .bytes()
            .take_while(|b| b.is_ascii_digit())
            .count();
        if digits == 0 || !lower[at + digits..].starts_with('"') {
            continue;
        }
        let id = &h[at..at + digits];
        let Some(gt) = h[at + digits..].find('>') else {
            continue;
        };
        let rest = h[at + digits + gt + 1..].trim_start();
        let run = rest
            .bytes()
            .take_while(|b| b.is_ascii_alphanumeric() || *b == b'.' || *b == b'-')
            .count();
        let name = &rest[..run];
        let Some((head, tld)) = name.rsplit_once('.') else {
            continue;
        };
        if head.is_empty() || tld.is_empty() || !tld.bytes().all(|b| b.is_ascii_alphabetic()) {
            continue;
        }
        if rest[run..].trim_start().starts_with('<') {
            out.push((id.to_string(), name.to_string()));
        }
    }
    out
}

/// The numeric id in `/bccs/<id>/edit`.
fn bcc_id(row: &str) -> Option<String> {
    let mut from = 0;
    while let Some(p) = row[from..].find("/bccs/") {
        let at = from + p + 6;
        from = at;
        let digits = row[at..].bytes().take_while(|b| b.is_ascii_digit()).count();
        if digits > 0 && row[at + digits..].starts_with("/edit") {
            return Some(row[at..at + digits].to_string());
        }
    }
    None
}

pub struct Web<'a> {
    cookie: String,
    http: &'a dyn Http,
}

const SPAM: &[(&str, &str, &str)] = &[
    ("aggressiveness", "edit", "domain[aggressiveness_level]"),
    (
        "subject_rewriting",
        "edit",
        "domain[subject_rewriting_enabled]",
    ),
    (
        "junk_subject_keyword",
        "edit",
        "domain[junk_subject_keyword_spam]",
    ),
    (
        "sender_denylist",
        "sender_denylist",
        "domain[sender_blacklist]",
    ),
    (
        "sender_junklist",
        "sender_denylist",
        "domain[sender_junklist]",
    ),
    (
        "sender_allowlist",
        "sender_allowlist",
        "domain[sender_whitelist]",
    ),
    (
        "allowlist_precedence",
        "sender_allowlist",
        "domain[sender_whitelist_precedence]",
    ),
    (
        "recipient_denylist",
        "recipient_denylist",
        "domain[recipient_blacklist]",
    ),
];
const LIST_KEYS: &[&str] = &[
    "sender_denylist",
    "sender_junklist",
    "sender_allowlist",
    "recipient_denylist",
];

fn spam_def(key: &str) -> Option<&'static (&'static str, &'static str, &'static str)> {
    SPAM.iter().find(|s| s.0 == key)
}

fn spam_templates<'k>(keys: impl Iterator<Item = &'k str>) -> Vec<&'static str> {
    let mut seen: Vec<&'static str> = Vec::new();
    for k in keys {
        if let Some(d) = spam_def(k) {
            if !seen.contains(&d.1) {
                seen.push(d.1);
            }
        }
    }
    seen
}

const BCC_FIELDS: &[(&str, &str)] = &[
    ("address", "blind_copy[local_part]"),
    ("destination", "blind_copy[bcc_local_part]"),
    ("direction", "blind_copy[direction]"),
];

impl<'a> Web<'a> {
    pub fn new(cookie: &str, http: &'a dyn Http) -> Self {
        Web {
            cookie: cookie.into(),
            http,
        }
    }

    fn page(&self, path: &str) -> Result<String> {
        let resp = self.http.send(Req {
            method: "GET".into(),
            url: format!("{PANEL}{path}"),
            headers: vec![("Cookie".into(), self.cookie.clone())],
            body: None,
        })?;
        let loc = resp.header("location").unwrap_or("");
        if (300..400).contains(&resp.status) && loc.contains("login") {
            return Err(
                "web session expired: sign in at admin.migadu.com and copy a fresh cookie".into(),
            );
        }
        if !(200..300).contains(&resp.status) {
            return Err(format!("GET {path} -> HTTP {}", resp.status));
        }
        Ok(resp.body)
    }

    /// POST a form the way the browser does. Fails when the panel answers an error or the login page.
    fn post(&self, action: &str, fields: &[(String, String)], referer: &str) -> Result<()> {
        let resp = self.http.send(Req {
            method: "POST".into(),
            url: format!("{PANEL}{action}"),
            headers: vec![
                ("Cookie".into(), self.cookie.clone()),
                (
                    "Content-Type".into(),
                    "application/x-www-form-urlencoded".into(),
                ),
                ("Referer".into(), format!("{PANEL}{referer}")),
            ],
            body: Some(form_urlencode(fields)),
        })?;
        if resp.status >= 400 || resp.header("location").unwrap_or("").contains("login") {
            return Err(format!("panel rejected the change (HTTP {})", resp.status));
        }
        Ok(())
    }

    pub fn plan(&self) -> Result<Map<String, Value>> {
        let h = self.page("/billing/subscription")?;
        let (name, billing) = parse_plan_header(&h)
            .ok_or("could not find the plan in the admin panel header (is the cookie valid?)")?;
        let mut m = Map::new();
        m.insert("plan".into(), json!(name));
        m.insert("billing".into(), json!(billing));
        m.insert(
            "renews".into(),
            parse_renews(&h).map_or(Value::Null, Value::String),
        );
        if let Some(l) = plan_limits(&name) {
            m.insert("limits".into(), l);
        }
        Ok(m)
    }

    fn domain_id(&self, name: &str) -> Result<String> {
        let want = name.to_lowercase();
        parse_domains(&self.page("/domains")?)
            .into_iter()
            .find(|(_, n)| *n == want)
            .map(|(id, _)| id)
            .ok_or_else(|| format!("domain \"{name}\" not found in the admin panel"))
    }

    // ----- catchall --------------------------------------------------------------------------

    fn read_catchall(&self, name: &str) -> Result<(String, String, Vec<String>)> {
        let id = self.domain_id(name)?;
        let h = self.page(&format!("/domains/{id}/catchall"))?;
        let ta = elements(&h, "textarea")
            .into_iter()
            .find(|e| attr(&e.attrs, "name").as_deref() == Some("domain[catchall_destinations]"));
        let csrf = start_tags(&h, "input")
            .iter()
            .find(|t| attr(t, "name").as_deref() == Some("_csrf_token"))
            .and_then(|t| attr(t, "value"))
            .filter(|v| !v.is_empty());
        match (ta, csrf) {
            (Some(ta), Some(csrf)) => Ok((id, csrf, tokens(&html::unescape(&ta.inner)))),
            _ => Err("catchall form not found (page layout changed or session invalid)".into()),
        }
    }

    pub fn get_catchall(&self, name: &str) -> Result<Value> {
        let (_, _, destinations) = self.read_catchall(name)?;
        Ok(json!({ "domain": name, "destinations": destinations }))
    }

    pub fn set_catchall(&self, name: &str, destinations: &[String]) -> Result<Value> {
        let (id, csrf, previous) = self.read_catchall(name)?;
        let fields = vec![
            ("_method".to_string(), "put".to_string()),
            ("_csrf_token".into(), csrf),
            ("template".into(), "catchall".into()),
            ("origin_path".into(), format!("/domains/{id}/catchall")),
            (
                "domain[catchall_destinations]".into(),
                destinations.join("\n"),
            ),
        ];
        self.post(
            &format!("/domains/{id}"),
            &fields,
            &format!("/domains/{id}/catchall"),
        )
        .map_err(|e| e.replace("panel rejected the change", "catchall update failed"))?;
        let after = self.read_catchall(name)?.2;
        // The panel keeps only the local part and re-attaches it to this domain, so compare by local part.
        let by_local = |l: &[String]| {
            l.iter()
                .map(|a| local_part(a).to_lowercase())
                .collect::<Vec<_>>()
                .join(",")
        };
        if by_local(&after) != by_local(destinations) {
            return Err(format!(
                "catchall update not applied; current value is [{}]",
                after.join(", ")
            ));
        }
        let mut out = json!({ "domain": name, "previous": previous, "destinations": after });
        if after.join(",") != destinations.join(",") {
            out["note"] = json!("the panel stores catchall recipients as addresses on this domain");
        }
        Ok(out)
    }

    // ----- spam filtering --------------------------------------------------------------------

    fn spam_form(&self, id: &str, template: &str) -> Result<Form> {
        let needle = format!("value=\"{template}\"");
        parse_form(
            &self.page(&format!("/domains/{id}/spam/{template}"))?,
            |body, _| body.contains(&needle),
        )
        .ok_or_else(|| {
            format!("spam form \"{template}\" not found (page layout changed or session invalid)")
        })
    }

    pub fn get_spam(&self, name: &str) -> Result<Map<String, Value>> {
        let id = self.domain_id(name)?;
        self.read_spam(name, &id)
    }

    fn read_spam(&self, name: &str, id: &str) -> Result<Map<String, Value>> {
        let mut out = Map::new();
        out.insert("domain".into(), json!(name));
        for template in spam_templates(SPAM.iter().map(|s| s.0)) {
            let form = self.spam_form(id, template)?;
            for (key, t, field) in SPAM {
                if *t != template {
                    continue;
                }
                let v = html::get_field(&form, field);
                if LIST_KEYS.contains(key) {
                    let text = match v {
                        Some(FieldValue::Text(t)) => t,
                        Some(FieldValue::Bool(b)) => b.to_string(),
                        None => String::new(),
                    };
                    out.insert((*key).into(), json!(tokens(&text)));
                } else if let Some(v) = field_value(v) {
                    out.insert((*key).into(), v);
                }
            }
        }
        Ok(out)
    }

    pub fn set_spam(&self, name: &str, changes: &Map<String, Value>) -> Result<Value> {
        for k in changes.keys() {
            if spam_def(k).is_none() {
                let all = SPAM.iter().map(|s| s.0).collect::<Vec<_>>().join(", ");
                return Err(format!("unknown spam setting \"{k}\" (use {all})"));
            }
        }
        let id = self.domain_id(name)?;
        let before = self.read_spam(name, &id)?;
        for template in spam_templates(changes.keys().map(|k| k.as_str())) {
            let mut form = self.spam_form(&id, template)?;
            for (k, v) in changes {
                let (_, t, field) = spam_def(k).expect("checked above");
                if *t == template {
                    let value = if LIST_KEYS.contains(&k.as_str()) {
                        tokens(&js_string(v)).join("\n")
                    } else {
                        js_string(v)
                    };
                    set_field(&mut form, field, &value, truthy(v));
                }
            }
            let action = form.action.clone().ok_or("spam form has no action")?;
            self.post(
                &action,
                &form.fields,
                &format!("/domains/{id}/spam/{template}"),
            )?;
        }
        let after = self.read_spam(name, &id)?;
        for (k, v) in changes {
            let list = LIST_KEYS.contains(&k.as_str());
            let want = if list {
                tokens(&js_string(v)).join(",")
            } else {
                js_string(v)
            };
            let got = match after.get(k) {
                Some(Value::Array(a)) if list => {
                    a.iter().map(js_string).collect::<Vec<_>>().join(",")
                }
                Some(x) => js_string(x),
                None => "undefined".into(),
            };
            if want != got {
                let cur = after
                    .get(k)
                    .map_or("undefined".to_string(), |x| x.to_string());
                return Err(format!(
                    "spam setting \"{k}\" not applied; current value is {cur}"
                ));
            }
        }
        let mut previous = Map::new();
        for k in changes.keys() {
            if let Some(v) = before.get(k) {
                previous.insert(k.clone(), v.clone());
            }
        }
        Ok(json!({ "domain": name, "previous": previous, "applied": changes }))
    }

    // ----- Bcc captures ----------------------------------------------------------------------

    pub fn list_bcc(&self, name: &str) -> Result<Vec<Map<String, Value>>> {
        let id = self.domain_id(name)?;
        self.read_bcc(&id)
    }

    fn read_bcc(&self, id: &str) -> Result<Vec<Map<String, Value>>> {
        let h = strip_comments(&self.page(&format!("/domains/{id}/bccs"))?);
        let mut out = Vec::new();
        for row in elements(&h, "tr") {
            let Some(bid) = bcc_id(&row.inner) else {
                continue;
            };
            let mut m = Map::new();
            m.insert("id".into(), json!(bid));
            let cells = elements(&row.inner, "td");
            for (key, cell) in ["address", "direction", "destination", "expires", "changed"]
                .iter()
                .zip(cells.iter())
            {
                m.insert((*key).into(), json!(html::text(&cell.inner)));
            }
            out.push(m);
        }
        Ok(out)
    }

    fn apply_bcc(form: &mut Form, fields: &[(String, Value)]) -> Result<()> {
        for (k, v) in fields {
            let Some((_, panel)) = BCC_FIELDS.iter().find(|(n, _)| n == k) else {
                return Err(format!(
                    "unknown bcc field \"{k}\" (use address, destination, direction)"
                ));
            };
            let s = js_string(v);
            if k == "direction" {
                if !["incoming", "outgoing", "all"].contains(&s.as_str()) {
                    return Err("direction must be incoming, outgoing or all".into());
                }
                set_field(form, panel, &s, false);
            } else {
                set_field(form, panel, local_part(&s), false);
            }
        }
        Ok(())
    }

    pub fn create_bcc(
        &self,
        name: &str,
        fields: &Map<String, Value>,
    ) -> Result<Map<String, Value>> {
        let given = |k: &str| fields.get(k).map(js_string).filter(|s| !s.is_empty());
        let (Some(address), Some(_)) = (given("address"), given("destination")) else {
            return Err("bcc create needs -f address=<pattern> -f destination=<address> [-f direction=incoming|outgoing|all]".into());
        };
        let id = self.domain_id(name)?;
        let mut form = parse_form(&self.page(&format!("/domains/{id}/bccs/new"))?, |b, _| {
            b.contains("blind_copy[")
        })
        .ok_or("bcc form not found (page layout changed or session invalid)")?;
        let mut ordered = vec![(
            "direction".to_string(),
            fields.get("direction").cloned().unwrap_or(json!("all")),
        )];
        ordered.extend(
            fields
                .iter()
                .filter(|(k, _)| *k != "direction")
                .map(|(k, v)| (k.clone(), v.clone())),
        );
        Self::apply_bcc(&mut form, &ordered)?;
        let action = form.action.clone().ok_or("bcc form has no action")?;
        self.post(&action, &form.fields, &format!("/domains/{id}/bccs/new"))?;
        let want = local_part(&address).to_string();
        self.read_bcc(&id)?
            .into_iter()
            .find(|b| local_part(b.get("address").and_then(Value::as_str).unwrap_or("")) == want)
            .ok_or_else(|| "bcc capture was not created".to_string())
    }

    fn find_bcc(&self, id: &str, name: &str, address: &str) -> Result<Map<String, Value>> {
        let want = local_part(address);
        self.read_bcc(id)?
            .into_iter()
            .find(|b| {
                let a = b.get("address").and_then(Value::as_str).unwrap_or("");
                a == address || local_part(a) == want
            })
            .ok_or_else(|| format!("no bcc capture \"{address}\" on {name}"))
    }

    pub fn update_bcc(
        &self,
        name: &str,
        address: &str,
        fields: &Map<String, Value>,
    ) -> Result<Value> {
        let id = self.domain_id(name)?;
        let b = self.find_bcc(&id, name, address)?;
        let bid = b["id"].as_str().unwrap_or("").to_string();
        let mut form = parse_form(
            &self.page(&format!("/domains/{id}/bccs/{bid}/edit"))?,
            |body, _| body.contains("blind_copy["),
        )
        .ok_or("bcc edit form not found")?;
        let ordered: Vec<(String, Value)> =
            fields.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        Self::apply_bcc(&mut form, &ordered)?;
        let action = form.action.clone().ok_or("bcc form has no action")?;
        self.post(
            &action,
            &form.fields,
            &format!("/domains/{id}/bccs/{bid}/edit"),
        )?;
        let new_address = fields
            .get("address")
            .map(js_string)
            .unwrap_or_else(|| address.to_string());
        Ok(json!({ "previous": b, "current": self.find_bcc(&id, name, &new_address)? }))
    }

    pub fn delete_bcc(&self, name: &str, address: &str) -> Result<Value> {
        let id = self.domain_id(name)?;
        let b = self.find_bcc(&id, name, address)?;
        let bid = b["id"].as_str().unwrap_or("").to_string();
        let mut form = parse_form(
            &self.page(&format!("/domains/{id}/bccs/{bid}/delete"))?,
            |body, _| body.contains("name=\"confirmation\""),
        )
        .ok_or("bcc delete form not found")?;
        // the panel asks for the full captured address
        set_field(
            &mut form,
            "confirmation",
            b.get("address").and_then(Value::as_str).unwrap_or(""),
            false,
        );
        let action = form.action.clone().ok_or("bcc form has no action")?;
        self.post(
            &action,
            &form.fields,
            &format!("/domains/{id}/bccs/{bid}/delete"),
        )?;
        if self
            .read_bcc(&id)?
            .iter()
            .any(|x| x.get("id") == b.get("id"))
        {
            return Err("bcc capture was not deleted".into());
        }
        Ok(json!({ "deleted": b }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scanners_read_the_panel_markup() {
        let h = r#"<a href="/billing/subscription">Mini / monthly</a> Active Plan renews November 27, 2026"#;
        assert_eq!(
            parse_plan_header(h),
            Some(("mini".into(), "monthly".into()))
        );
        assert_eq!(parse_renews(h).as_deref(), Some("November 27, 2026"));
        assert_eq!(
            parse_plan_header("<a href=\"/billing/subscription\">Sign in</a>"),
            None
        );
        let d = r#"<a href="/domains/9">example.org</a> <a href="/domains/12" class="x"> Sub.Example.COM </a> <a href="/domains/5">nodot</a>"#;
        assert_eq!(
            parse_domains(d),
            vec![
                ("9".into(), "example.org".into()),
                ("12".into(), "Sub.Example.COM".into())
            ]
        );
        assert_eq!(
            bcc_id(r#"<a href="/domains/9/bccs/17/edit">x</a>"#).as_deref(),
            Some("17")
        );
        assert_eq!(bcc_id(r#"<a href="/domains/9/bccs/new">x</a>"#), None);
    }

    #[test]
    fn form_encoding_matches_browsers() {
        let f = vec![
            ("a b".to_string(), "x&y=z".to_string()),
            ("domain[x]".into(), "one\ntwo".into()),
        ];
        assert_eq!(form_urlencode(&f), "a+b=x%26y%3Dz&domain%5Bx%5D=one%0Atwo");
    }
}
