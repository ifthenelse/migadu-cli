//! Tiny HTML helpers and form reader.
//!
//! Returns what a browser would submit for a form, so a write can resend untouched fields.
//! This is not a general HTML parser: it scans Migadu's server-rendered markup and will need
//! updating if the panel changes (see docs/runbooks/panel-markup-changed.md).

use std::collections::HashSet;

pub fn unescape(t: &str) -> String {
    t.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&#x27;", "'")
        .replace("&amp;", "&")
}

fn is_word(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Drop HTML comments (commented-out controls are not part of the form), repeating until
/// stable so nested fragments such as `<!<!---->--` cannot reassemble, and discarding any
/// unterminated `<!--` tail.
pub fn strip_comments(html: &str) -> String {
    let mut s = html.to_string();
    while let Some(start) = s.find("<!--") {
        match s[start + 4..].find("-->") {
            Some(end) => s.replace_range(start..start + 4 + end + 3, ""),
            None => {
                s.truncate(start);
                break;
            }
        }
    }
    s
}

/// Value of `name="..."` or `name='...'` inside a tag's attribute text (case-insensitive name).
pub fn attr(tag: &str, name: &str) -> Option<String> {
    let lower = tag.to_ascii_lowercase();
    let name = name.to_ascii_lowercase();
    let bytes = lower.as_bytes();
    let mut from = 0;
    while let Some(p) = lower[from..].find(&name) {
        let at = from + p;
        from = at + name.len();
        // a whole attribute name: not the tail of `data-name`, `x:name` or `_name`
        if at > 0 && (is_word(bytes[at - 1]) || bytes[at - 1] == b'-' || bytes[at - 1] == b':') {
            continue;
        }
        let mut i = at + name.len();
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if bytes.get(i) != Some(&b'=') {
            continue;
        }
        i += 1;
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        let Some(&q) = bytes.get(i) else { continue };
        if q != b'"' && q != b'\'' {
            continue;
        }
        if let Some(len) = tag[i + 1..].find(q as char) {
            return Some(unescape(&tag[i + 1..i + 1 + len]));
        }
    }
    None
}

/// True when `word` appears as a whole word (`\bword\b`, case-insensitive), e.g. `disabled`.
pub fn has_word(tag: &str, word: &str) -> bool {
    let lower = tag.to_ascii_lowercase();
    let bytes = lower.as_bytes();
    let mut from = 0;
    while let Some(p) = lower[from..].find(word) {
        let at = from + p;
        from = at + word.len();
        let before = at == 0 || !is_word(bytes[at - 1]);
        let after = at + word.len() >= bytes.len() || !is_word(bytes[at + word.len()]);
        if before && after {
            return true;
        }
    }
    false
}

/// An element: its attribute text and inner HTML.
pub struct Element {
    pub attrs: String,
    pub inner: String,
}

/// Position of the next `<name ...>` start tag at or after `from`: (tag start, attrs, end of tag).
fn next_open(lower: &str, html: &str, name: &str, from: usize) -> Option<(usize, String, usize)> {
    let pat = format!("<{name}");
    let mut at = from;
    while let Some(p) = lower[at..].find(&pat) {
        let start = at + p;
        let after = start + pat.len();
        at = after;
        if lower.as_bytes().get(after).is_some_and(|b| is_word(*b)) {
            continue;
        }
        let gt = html[after..].find('>')?;
        return Some((start, html[after..after + gt].to_string(), after + gt + 1));
    }
    None
}

/// All `<name ...>inner</name>` elements, non-overlapping, in document order.
pub fn elements(html: &str, name: &str) -> Vec<Element> {
    let lower = html.to_ascii_lowercase();
    let close = format!("</{name}>");
    let mut out = Vec::new();
    let mut pos = 0;
    while let Some((_, attrs, body)) = next_open(&lower, html, name, pos) {
        let Some(c) = lower[body..].find(&close) else {
            break;
        };
        out.push(Element {
            attrs,
            inner: html[body..body + c].to_string(),
        });
        pos = body + c + close.len();
    }
    out
}

/// Attribute text of every `<name ...>` start tag.
pub fn start_tags(html: &str, name: &str) -> Vec<String> {
    let lower = html.to_ascii_lowercase();
    let mut out = Vec::new();
    let mut pos = 0;
    while let Some((_, attrs, end)) = next_open(&lower, html, name, pos) {
        out.push(attrs);
        pos = end;
    }
    out
}

/// Remove tags, unescape, collapse whitespace.
pub fn text(h: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for c in h.chars() {
        match c {
            '<' => {
                in_tag = true;
                out.push(' ');
            }
            '>' if in_tag => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    unescape(&out)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// What a browser would submit for one `<form>`.
#[derive(Debug, Clone)]
pub struct Form {
    pub action: Option<String>,
    pub fields: Vec<(String, String)>,
    pub checkboxes: HashSet<String>,
}

/// Read the first form for which `pick(body, tag_attrs)` is true.
pub fn parse_form(html: &str, pick: impl Fn(&str, &str) -> bool) -> Option<Form> {
    let html = strip_comments(html);
    let form = elements(&html, "form")
        .into_iter()
        .find(|f| pick(&f.inner, &f.attrs))?;
    let body = form.inner.as_str();
    let lower = body.to_ascii_lowercase();
    let mut fields: Vec<(String, String)> = Vec::new();
    let mut checkboxes = HashSet::new();
    let mut pos = 0;
    loop {
        // next input/select/textarea start tag
        let next = ["input", "select", "textarea"]
            .iter()
            .filter_map(|k| next_open(&lower, body, k, pos).map(|(s, a, e)| (s, *k, a, e)))
            .min_by_key(|(s, ..)| *s);
        let Some((_, kind, tag, end)) = next else {
            break;
        };
        pos = end;
        let mut inner = String::new();
        if kind != "input" {
            let close = format!("</{kind}>");
            if let Some(c) = lower[end..].find(&close) {
                inner = body[end..end + c].to_string();
                pos = end + c + close.len();
            }
        }
        let Some(name) = attr(&tag, "name") else {
            continue;
        };
        if name.is_empty() || has_word(&tag, "disabled") {
            continue;
        }
        match kind {
            "textarea" => {
                let v = unescape(&inner);
                let v = v
                    .strip_prefix("\r\n")
                    .or_else(|| v.strip_prefix('\n'))
                    .unwrap_or(&v)
                    .to_string();
                fields.push((name, v));
            }
            "select" => {
                let opts: Vec<(String, bool)> = start_tags(&inner, "option")
                    .iter()
                    .map(|o| {
                        (
                            attr(o, "value").unwrap_or_default(),
                            has_word(o, "selected"),
                        )
                    })
                    .collect();
                if let Some((v, _)) = opts.iter().find(|o| o.1).or(opts.first()) {
                    fields.push((name, v.clone()));
                }
            }
            _ => {
                let ty = attr(&tag, "type")
                    .unwrap_or_else(|| "text".into())
                    .to_ascii_lowercase();
                if ["submit", "button", "file", "image", "reset"].contains(&ty.as_str()) {
                    continue;
                }
                let boxy = ty == "checkbox" || ty == "radio";
                if ty == "checkbox" {
                    checkboxes.insert(name.clone());
                }
                if boxy && !has_word(&tag, "checked") {
                    continue;
                }
                fields.push((
                    name,
                    attr(&tag, "value").unwrap_or_else(|| {
                        if boxy {
                            "on".into()
                        } else {
                            String::new()
                        }
                    }),
                ));
            }
        }
    }
    Some(Form {
        action: attr(&form.attrs, "action"),
        fields,
        checkboxes,
    })
}

/// Replace a field's value; checkboxes follow Rails' hidden "false" + checked "true" pair.
pub fn set_field(form: &mut Form, name: &str, value: &str, truthy: bool) {
    form.fields.retain(|(n, _)| n != name);
    if form.checkboxes.contains(name) {
        form.fields.push((name.into(), "false".into()));
        if truthy {
            form.fields.push((name.into(), "true".into()));
        }
    } else {
        form.fields.push((name.into(), value.into()));
    }
}

/// A checkbox reads as true when any submitted value is "true"; other fields give their last value.
pub fn get_field(form: &Form, name: &str) -> Option<FieldValue> {
    let vals: Vec<&String> = form
        .fields
        .iter()
        .filter(|(n, _)| n == name)
        .map(|(_, v)| v)
        .collect();
    if form.checkboxes.contains(name) {
        Some(FieldValue::Bool(vals.iter().any(|v| v.as_str() == "true")))
    } else {
        vals.last().map(|v| FieldValue::Text((*v).clone()))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum FieldValue {
    Bool(bool),
    Text(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    const FORM: &str = r#"<form action="/domains/9/spam" method="post">
<input type="hidden" name="_method" value="put"><input type="hidden" name="_csrf_token" value="t&amp;k">
<input type="hidden" name="template" value="edit">
<select name="domain[aggressiveness_level]"><option value="3">a</option><option value="0" selected>b</option></select>
<input type="hidden" name="domain[subject_rewriting_enabled]" value="false"><input type="checkbox" name="domain[subject_rewriting_enabled]" value="true" checked>
<!-- <input type="checkbox" name="domain[greylisting_enabled]" value="true" checked> -->
<textarea name="domain[x]">
one
two</textarea><input type="submit" name="go" value="Save"></form>"#;

    fn pair(a: &str, b: &str) -> (String, String) {
        (a.to_string(), b.to_string())
    }

    #[test]
    fn parse_form_returns_what_a_browser_would_submit() {
        let mut f = parse_form(FORM, |b, _| b.contains("value=\"edit\"")).unwrap();
        assert_eq!(
            f.fields,
            vec![
                pair("_method", "put"),
                pair("_csrf_token", "t&k"),
                pair("template", "edit"),
                pair("domain[aggressiveness_level]", "0"),
                pair("domain[subject_rewriting_enabled]", "false"),
                pair("domain[subject_rewriting_enabled]", "true"),
                pair("domain[x]", "one\ntwo"),
            ]
        );
        assert_eq!(f.action.as_deref(), Some("/domains/9/spam"));
        assert_eq!(
            get_field(&f, "domain[subject_rewriting_enabled]"),
            Some(FieldValue::Bool(true))
        );
        set_field(&mut f, "domain[subject_rewriting_enabled]", "false", false);
        assert_eq!(
            get_field(&f, "domain[subject_rewriting_enabled]"),
            Some(FieldValue::Bool(false))
        );
        set_field(&mut f, "domain[aggressiveness_level]", "3", false);
        assert_eq!(
            get_field(&f, "domain[aggressiveness_level]"),
            Some(FieldValue::Text("3".into()))
        );
    }

    #[test]
    fn comments_cannot_reassemble() {
        assert_eq!(strip_comments("a<!-- x -->b"), "ab");
        assert!(!strip_comments("<!<!---->-- <input name=\"x\">").contains("<!--"));
        assert_eq!(strip_comments("keep<!-- unterminated"), "keep");
    }

    #[test]
    fn attr_matches_whole_names_only() {
        assert_eq!(
            attr(r#" data-name="no" name="yes""#, "name").as_deref(),
            Some("yes")
        );
        assert_eq!(attr(r#" value='a&amp;b'"#, "value").as_deref(), Some("a&b"));
        assert_eq!(attr(" disabled", "value"), None);
    }
}
