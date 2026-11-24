//! Fake admin panels (catchall, spam, Bcc) that apply form posts the way the real one does.
mod common;
use common::*;
use migadu::http::{Req, Resp};
use serde_json::json;
use std::cell::RefCell;

const COOKIE: &[(&str, &str)] = &[("MIGADU_WEB_COOKIE", "s=1")];
const DOMAINS: &str = r#"<a href="/domains/9">example.org</a>"#;

fn run(argv: &[&str], h: &Fake) -> Out {
    run_cli(argv, &env(COOKIE), h, &FakeExec::new())
}

// ----- catchall ----------------------------------------------------------------------------------

struct Catchall {
    value: RefCell<String>,
    apply: bool,
    rewrite_to: Option<&'static str>,
}

impl Catchall {
    fn new(initial: &str) -> Self {
        Catchall {
            value: RefCell::new(initial.into()),
            apply: true,
            rewrite_to: None,
        }
    }
    fn handle(&self, req: &Req) -> Resp {
        if req.method == "POST" {
            let form = decode(req.body.as_deref().unwrap());
            if self.apply {
                let mut v = get(&form, "domain[catchall_destinations]")
                    .unwrap()
                    .to_string();
                if let Some(d) = self.rewrite_to {
                    v = v
                        .lines()
                        .map(|l| format!("{}@{d}", l.split('@').next().unwrap()))
                        .collect::<Vec<_>>()
                        .join("\n");
                }
                *self.value.borrow_mut() = v;
            }
            return redirect("/domains/9/catchall");
        }
        if req.url.ends_with("/domains") {
            return resp(200, DOMAINS);
        }
        resp(
            200,
            &format!(
                r#"<form><input type="hidden" name="_csrf_token" value="tok&amp;1"><textarea name="domain[catchall_destinations]">{}</textarea></form>"#,
                self.value.borrow()
            ),
        )
    }
}

#[test]
fn catchall_get_lists_destinations() {
    let p = Catchall::new("a@x.org\nb@x.org");
    let h = Fake::new(|r| p.handle(r));
    assert_eq!(
        run(&["catchall", "get", "example.org"], &h).json()["destinations"],
        json!(["a@x.org", "b@x.org"])
    );
}

#[test]
fn catchall_set_posts_the_panels_put_form_with_csrf_and_returns_the_previous_value() {
    let p = Catchall::new("old@x.org");
    let h = Fake::new(|r| p.handle(r));
    let r = run(
        &[
            "catchall",
            "set",
            "example.org",
            "-f",
            "destinations=a@x.org,b@x.org",
        ],
        &h,
    );
    assert_eq!(r.code, 0, "{}", r.stderr());
    let calls = h.calls.borrow();
    let post = calls.iter().find(|c| c.method == "POST").unwrap();
    assert_eq!(post.url, "https://admin.migadu.com/domains/9");
    assert_eq!(post.header("cookie"), Some("s=1"));
    assert_eq!(
        post.header("referer"),
        Some("https://admin.migadu.com/domains/9/catchall")
    );
    let form = post.form();
    assert_eq!(get(&form, "_method"), Some("put"));
    assert_eq!(get(&form, "_csrf_token"), Some("tok&1"));
    assert_eq!(get(&form, "template"), Some("catchall"));
    assert_eq!(get(&form, "origin_path"), Some("/domains/9/catchall"));
    assert_eq!(
        get(&form, "domain[catchall_destinations]"),
        Some("a@x.org\nb@x.org")
    );
    assert_eq!(r.json()["previous"], json!(["old@x.org"]));
}

#[test]
fn catchall_set_fails_loudly_when_the_panel_did_not_apply_it_and_clearing_needs_yes() {
    let p = Catchall {
        apply: false,
        ..Catchall::new("")
    };
    let h = Fake::new(|r| p.handle(r));
    assert!(run(
        &[
            "catchall",
            "set",
            "example.org",
            "-f",
            "destinations=a@x.org"
        ],
        &h
    )
    .stderr()
    .contains("not applied"));
    let p2 = Catchall::new("");
    let h2 = Fake::new(|r| p2.handle(r));
    assert!(run(&["catchall", "set", "example.org"], &h2)
        .stderr()
        .contains("--yes"));
}

#[test]
fn catchall_set_accepts_the_panel_rewriting_recipients_onto_the_domain() {
    let p = Catchall {
        rewrite_to: Some("example.org"),
        ..Catchall::new("")
    };
    let h = Fake::new(|r| p.handle(r));
    let r = run(
        &[
            "catchall",
            "set",
            "example.org",
            "-f",
            "destinations=admin@other.com",
        ],
        &h,
    );
    assert_eq!(r.code, 0, "{}", r.stderr());
    let j = r.json();
    assert_eq!(j["destinations"], json!(["admin@example.org"]));
    assert!(j["note"].as_str().unwrap().contains("this domain"));
}

// ----- spam --------------------------------------------------------------------------------------

#[derive(Clone)]
struct SpamState {
    aggr: String,
    subj: bool,
    junk: bool,
    deny: String,
    junklist: String,
    allow: String,
    prec: bool,
    rcpt: String,
}

struct Spam {
    v: RefCell<SpamState>,
    apply: bool,
}

fn cb(n: &str, on: bool) -> String {
    format!(
        r#"<input type="hidden" name="{n}" value="false"><input type="checkbox" name="{n}" value="true"{}>"#,
        if on { " checked" } else { "" }
    )
}

fn head(template: &str) -> String {
    format!(
        r#"<form action="/domains/9/spam"><input type="hidden" name="_method" value="put"><input type="hidden" name="_csrf_token" value="c"><input type="hidden" name="template" value="{template}">"#
    )
}

impl Spam {
    fn new() -> Self {
        Spam {
            v: RefCell::new(SpamState {
                aggr: "0".into(),
                subj: false,
                junk: false,
                deny: "".into(),
                junklist: "".into(),
                allow: "".into(),
                prec: false,
                rcpt: "".into(),
            }),
            apply: true,
        }
    }
    fn handle(&self, req: &Req) -> Resp {
        if req.method == "POST" {
            if self.apply {
                let b = decode(req.body.as_deref().unwrap());
                let mut v = self.v.borrow_mut();
                let flag = |k: &str| all(&b, k).contains(&"true");
                match get(&b, "template").unwrap() {
                    "edit" => {
                        v.aggr = get(&b, "domain[aggressiveness_level]").unwrap().into();
                        v.subj = flag("domain[subject_rewriting_enabled]");
                        v.junk = flag("domain[junk_subject_keyword_spam]");
                    }
                    "sender_denylist" => {
                        v.deny = get(&b, "domain[sender_blacklist]").unwrap().into();
                        v.junklist = get(&b, "domain[sender_junklist]").unwrap().into();
                    }
                    "sender_allowlist" => {
                        v.allow = get(&b, "domain[sender_whitelist]").unwrap().into();
                        v.prec = flag("domain[sender_whitelist_precedence]");
                    }
                    "recipient_denylist" => {
                        v.rcpt = get(&b, "domain[recipient_blacklist]").unwrap().into()
                    }
                    t => panic!("unexpected template {t}"),
                }
            }
            return redirect("/domains/9/spam");
        }
        if req.url.ends_with("/domains") {
            return resp(200, DOMAINS);
        }
        let v = self.v.borrow();
        let html = match req.url.rsplit('/').next().unwrap() {
            "edit" => {
                let opts: String = ["3", "0", "-3"].iter().map(|o| format!(r#"<option value="{o}"{}>x</option>"#, if *o == v.aggr { " selected" } else { "" })).collect();
                format!("{}<select name=\"domain[aggressiveness_level]\">{opts}</select>{}{}</form>", head("edit"), cb("domain[subject_rewriting_enabled]", v.subj), cb("domain[junk_subject_keyword_spam]", v.junk))
            }
            "sender_denylist" => format!("{}<textarea name=\"domain[sender_blacklist]\">{}</textarea><textarea name=\"domain[sender_junklist]\">{}</textarea></form>", head("sender_denylist"), v.deny, v.junklist),
            "sender_allowlist" => format!("{}<textarea name=\"domain[sender_whitelist]\">{}</textarea>{}</form>", head("sender_allowlist"), v.allow, cb("domain[sender_whitelist_precedence]", v.prec)),
            "recipient_denylist" => format!("{}<textarea name=\"domain[recipient_blacklist]\">{}</textarea></form>", head("recipient_denylist"), v.rcpt),
            other => panic!("unexpected page {other}"),
        };
        resp(200, &html)
    }
}

#[test]
fn spam_get_reads_all_four_panel_forms() {
    let p = Spam::new();
    let h = Fake::new(|r| p.handle(r));
    assert_eq!(
        run(&["spam", "get", "example.org"], &h).json(),
        json!({ "domain": "example.org", "aggressiveness": "0", "subject_rewriting": false, "junk_subject_keyword": false,
                "sender_denylist": [], "sender_junklist": [], "sender_allowlist": [], "allowlist_precedence": false, "recipient_denylist": [] })
    );
}

#[test]
fn spam_set_changes_only_what_was_asked_and_preserves_sibling_fields_in_the_same_form() {
    let p = Spam::new();
    p.v.borrow_mut().junk = true;
    let h = Fake::new(|r| p.handle(r));
    let r = run(
        &[
            "spam",
            "set",
            "example.org",
            "-f",
            "aggressiveness=-3",
            "-f",
            "subject_rewriting:=true",
            "-f",
            "sender_denylist=a@x.org,b@x.org",
        ],
        &h,
    );
    assert_eq!(r.code, 0, "{}", r.stderr());
    let v = p.v.borrow();
    assert_eq!(
        (v.aggr.as_str(), v.subj, v.junk, v.deny.as_str()),
        ("-3", true, true, "a@x.org\nb@x.org")
    );
    assert_eq!(
        r.json()["previous"],
        json!({ "aggressiveness": "0", "subject_rewriting": false, "sender_denylist": [] })
    );
}

#[test]
fn spam_set_rejects_unknown_keys_and_reports_changes_the_panel_did_not_apply() {
    let p = Spam::new();
    let h = Fake::new(|r| p.handle(r));
    assert!(run(&["spam", "set", "example.org", "-f", "bogus=1"], &h)
        .stderr()
        .contains("unknown spam setting"));
    let p2 = Spam {
        apply: false,
        ..Spam::new()
    };
    let h2 = Fake::new(|r| p2.handle(r));
    assert!(run(
        &["spam", "set", "example.org", "-f", "aggressiveness=3"],
        &h2
    )
    .stderr()
    .contains("not applied"));
}

// ----- Bcc ---------------------------------------------------------------------------------------

struct Row {
    id: u32,
    a: String,
    t: String,
    d: String,
}

struct Bcc {
    rows: RefCell<Vec<Row>>,
    next: RefCell<u32>,
}

impl Bcc {
    fn new() -> Self {
        Bcc {
            rows: RefCell::new(vec![]),
            next: RefCell::new(1),
        }
    }
    fn handle(&self, req: &Req) -> Resp {
        let url = req.url.as_str();
        if req.method == "POST" {
            let b = decode(req.body.as_deref().unwrap());
            let mut rows = self.rows.borrow_mut();
            if url.ends_with("/bccs") {
                let id = *self.next.borrow();
                *self.next.borrow_mut() += 1;
                rows.push(Row {
                    id,
                    a: get(&b, "blind_copy[local_part]").unwrap().into(),
                    t: get(&b, "blind_copy[bcc_local_part]").unwrap().into(),
                    d: get(&b, "blind_copy[direction]").unwrap().into(),
                });
            } else if get(&b, "_method") == Some("delete") {
                if get(&b, "confirmation") == Some(&format!("{}@example.org", rows[0].a)) {
                    rows.clear();
                }
            } else {
                let r = &mut rows[0];
                r.a = get(&b, "blind_copy[local_part]").unwrap().into();
                r.t = get(&b, "blind_copy[bcc_local_part]").unwrap().into();
                r.d = get(&b, "blind_copy[direction]").unwrap().into();
            }
            return redirect("/domains/9/bccs");
        }
        let rows = self.rows.borrow();
        let html = if url.ends_with("/domains") {
            DOMAINS.to_string()
        } else if url.ends_with("/bccs/new") {
            r#"<form action="/domains/9/bccs" method="post"><input type="hidden" name="_csrf_token" value="c"><input type="hidden" name="blind_copy[tags]" value=""><input name="blind_copy[local_part]"><input name="blind_copy[bcc_local_part]"><select name="blind_copy[direction]"><option value="incoming">i</option><option value="outgoing">o</option><option value="all">a</option></select></form>"#.to_string()
        } else if url.ends_with("/delete") {
            r#"<form action="/domains/9/bccs/1" method="post"><input type="hidden" name="_method" value="delete"><input type="hidden" name="_csrf_token" value="c"><input type="text" name="confirmation"></form>"#.to_string()
        } else if url.ends_with("/edit") {
            format!(
                r#"<form action="/domains/9/bccs/1" method="post"><input type="hidden" name="_method" value="put"><input type="hidden" name="_csrf_token" value="c"><input name="blind_copy[local_part]" value="{}"><input name="blind_copy[bcc_local_part]" value="{}"><select name="blind_copy[direction]"><option value="incoming" selected>i</option><option value="all">a</option></select></form>"#,
                rows[0].a, rows[0].t
            )
        } else {
            let body: String = rows.iter().map(|r| format!(r#"<tr><td><a href="/domains/9/bccs/{id}/edit">{a}@example.org</a></td><td>{d}</td><td>{t}@example.org</td><td><a href="/domains/9/bccs/{id}/expire">Never</a></td><td>02:20</td><td><a href="/domains/9/bccs/{id}/edit">Edit</a> | <a href="/domains/9/bccs/{id}/delete">Delete...</a></td></tr>"#, id = r.id, a = r.a, d = r.d, t = r.t)).collect();
            format!("<table>{body}</table>")
        };
        resp(200, &html)
    }
}

#[test]
fn bcc_create_list_update_and_delete_follow_the_panels_forms() {
    let p = Bcc::new();
    let h = Fake::new(|r| p.handle(r));
    let mut r = run(
        &[
            "bcc",
            "create",
            "-d",
            "example.org",
            "-f",
            "address=audit@example.org",
            "-f",
            "destination=archive",
            "-f",
            "direction=incoming",
        ],
        &h,
    );
    assert_eq!(r.code, 0, "{}", r.stderr());
    {
        let calls = h.calls.borrow();
        let post = calls.iter().find(|c| c.method == "POST").unwrap();
        assert_eq!(post.url, "https://admin.migadu.com/domains/9/bccs");
        let f = post.form();
        assert_eq!(
            (
                get(&f, "_csrf_token"),
                get(&f, "blind_copy[tags]"),
                get(&f, "blind_copy[local_part]"),
                get(&f, "blind_copy[bcc_local_part]"),
                get(&f, "blind_copy[direction]")
            ),
            (
                Some("c"),
                Some(""),
                Some("audit"),
                Some("archive"),
                Some("incoming")
            )
        );
    }
    r = run(&["bcc", "list", "-d", "example.org"], &h);
    let rows = r.json();
    let row = &rows[0];
    assert_eq!(
        (
            &row["id"],
            &row["address"],
            &row["direction"],
            &row["destination"]
        ),
        (
            &json!("1"),
            &json!("audit@example.org"),
            &json!("incoming"),
            &json!("archive@example.org")
        )
    );
    r = run(
        &[
            "bcc",
            "update",
            "audit",
            "-d",
            "example.org",
            "-f",
            "destination=other",
        ],
        &h,
    );
    assert_eq!(r.code, 0, "{}", r.stderr());
    assert_eq!(r.json()["current"]["destination"], "other@example.org");
    assert!(run(&["bcc", "delete", "audit", "-d", "example.org"], &h)
        .stderr()
        .contains("--yes"));
    r = run(
        &["bcc", "delete", "audit", "-d", "example.org", "--yes"],
        &h,
    );
    assert_eq!(r.code, 0, "{}", r.stderr());
    assert_eq!(
        run(&["bcc", "list", "-d", "example.org"], &h).json(),
        json!([])
    );
}

#[test]
fn bcc_validates_direction_and_requires_address_and_destination() {
    let p = Bcc::new();
    let h = Fake::new(|r| p.handle(r));
    assert!(run(
        &["bcc", "create", "-d", "example.org", "-f", "address=a"],
        &h
    )
    .stderr()
    .contains("needs -f address"));
    assert!(run(
        &[
            "bcc",
            "create",
            "-d",
            "example.org",
            "-f",
            "address=a",
            "-f",
            "destination=b",
            "-f",
            "direction=sideways"
        ],
        &h
    )
    .stderr()
    .contains("direction must be"));
}
