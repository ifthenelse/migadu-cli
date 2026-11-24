mod common;
use common::*;
use migadu::secrets::resolve_secret;
use serde_json::json;

const ENV: &[(&str, &str)] = &[
    ("MIGADU_EMAIL", "a@x.org"),
    ("MIGADU_API_KEY", "k"),
    ("MIGADU_DOMAIN", "x.org"),
];

fn ok_empty() -> Fake<'static> {
    Fake::new(|_| json_resp(200, json!({})))
}

#[test]
fn mailboxes_list_uses_basic_auth_and_domain_path() {
    let h = Fake::new(|_| json_resp(200, json!({ "mailboxes": [{ "address": "b@x.org" }] })));
    let r = run_cli(&["mailboxes", "list"], &env(ENV), &h, &FakeExec::new());
    assert_eq!(r.code, 0, "{}", r.stderr());
    let calls = h.calls.borrow();
    assert_eq!(
        calls[0].url,
        "https://api.migadu.com/v1/domains/x.org/mailboxes"
    );
    assert_eq!(calls[0].header("authorization"), Some("Basic YUB4Lm9yZzpr"));
}

#[test]
fn create_builds_body_from_field_with_typed_values() {
    let h = ok_empty();
    run_cli(
        &[
            "mailboxes",
            "create",
            "-f",
            "local_part=bob",
            "-f",
            "password=12345678",
            "-f",
            "may_send:=true",
        ],
        &env(ENV),
        &h,
        &FakeExec::new(),
    );
    let calls = h.calls.borrow();
    assert_eq!(
        calls[0].json(),
        json!({ "local_part": "bob", "password": "12345678", "may_send": true })
    );
    assert_eq!(calls[0].method, "POST");
}

#[test]
fn json_body_from_stdin_and_field_overrides() {
    let h = ok_empty();
    run_cli(
        &[
            "api",
            "POST",
            "domains/x.org/rewrites",
            "--json",
            "-",
            "-f",
            "b=2",
        ],
        &env(ENV),
        &h,
        &FakeExec::new(),
    );
    let calls = h.calls.borrow();
    assert_eq!(
        calls[0].url,
        "https://api.migadu.com/v1/domains/x.org/rewrites"
    );
    assert_eq!(calls[0].json(), json!({ "a": 1, "b": "2" }));
}

#[test]
fn scoped_resources_put_the_mailbox_in_the_path() {
    let h = ok_empty();
    run_cli(
        &["forwardings", "get", "bob", "c@y.org"],
        &env(ENV),
        &h,
        &FakeExec::new(),
    );
    assert_eq!(
        h.urls()[0],
        "https://api.migadu.com/v1/domains/x.org/mailboxes/bob/forwardings/c%40y.org"
    );
}

#[test]
fn delete_needs_yes() {
    let h = ok_empty();
    let r = run_cli(
        &["aliases", "delete", "info"],
        &env(ENV),
        &h,
        &FakeExec::new(),
    );
    assert_eq!(r.code, 1);
    assert!(r.stderr().contains("--yes"));
    assert_eq!(h.calls.borrow().len(), 0);
    assert_eq!(
        run_cli(
            &["aliases", "delete", "info", "--yes"],
            &env(ENV),
            &h,
            &FakeExec::new()
        )
        .code,
        0
    );
}

#[test]
fn delete_treats_http_500_followed_by_404_as_success() {
    let h = Fake::new(|req| {
        if req.method == "DELETE" {
            json_resp(500, json!({}))
        } else {
            json_resp(404, json!({}))
        }
    });
    let r = run_cli(
        &["mailboxes", "delete", "bob", "-y"],
        &env(ENV),
        &h,
        &FakeExec::new(),
    );
    assert_eq!(r.code, 0, "{}", r.stderr());
    assert_eq!(r.json(), json!({ "deleted": true }));
}

#[test]
fn http_errors_carry_the_api_message() {
    let h = Fake::new(|_| json_resp(422, json!({ "error": "local_part is taken" })));
    let r = run_cli(
        &["mailboxes", "create", "-f", "local_part=bob"],
        &env(ENV),
        &h,
        &FakeExec::new(),
    );
    assert_eq!(r.code, 1);
    assert_eq!(
        r.stderr(),
        "error: POST /domains/x.org/mailboxes -> HTTP 422: local_part is taken"
    );
}

#[test]
fn domain_actions_map_to_endpoints() {
    let h = ok_empty();
    run_cli(
        &["domains", "activate", "x.org"],
        &env(ENV),
        &h,
        &FakeExec::new(),
    );
    let calls = h.calls.borrow();
    assert_eq!(
        (calls[0].method.as_str(), calls[0].url.as_str()),
        ("POST", "https://api.migadu.com/v1/domains/x.org/activate")
    );
}

#[test]
fn tier_reports_plan_fields_when_present_and_a_note_when_absent() {
    let h = Fake::new(|req| {
        if req.url.ends_with("/domains") {
            json_resp(
                200,
                json!({ "domains": [{ "name": "x.org", "active": true, "plan": "standard" }] }),
            )
        } else {
            json_resp(200, json!({ "mailboxes": 3 }))
        }
    });
    let r = run_cli(&["tier"], &env(ENV), &h, &FakeExec::new());
    assert_eq!(r.json()["plan_fields"]["x.org.plan"], "standard");
    let h2 = Fake::new(|req| {
        if req.url.ends_with("/domains") {
            json_resp(200, json!({ "domains": [{ "name": "x.org" }] }))
        } else {
            json_resp(200, json!({}))
        }
    });
    let r2 = run_cli(&["tier"], &env(ENV), &h2, &FakeExec::new());
    assert!(r2.json()["note"].as_str().unwrap().contains("no plan/tier"));
}

#[test]
fn missing_credentials_fail_with_a_clear_message() {
    let h = ok_empty();
    let r = run_cli(&["domains", "list"], &env(&[]), &h, &FakeExec::new());
    assert_eq!(r.code, 1);
    assert!(r.stderr().contains("MIGADU_EMAIL"));
}

#[test]
fn bad_options_exit_with_code_2() {
    let h = ok_empty();
    assert_eq!(
        run_cli(&["--nope"], &env(ENV), &h, &FakeExec::new()).code,
        2
    );
    assert_eq!(run_cli(&[], &env(ENV), &h, &FakeExec::new()).code, 2);
    let help = run_cli(&["--help"], &env(ENV), &h, &FakeExec::new());
    assert_eq!(help.code, 0);
    assert!(help.out[0].starts_with("migadu — Migadu email hosting"));
}

#[test]
fn table_output_lists_scalar_columns() {
    let h = Fake::new(|_| {
        json_resp(
            200,
            json!({ "mailboxes": [{ "address": "b@x.org", "n": 3, "o": {} }, { "address": "cc@x.org", "n": 10 }] }),
        )
    });
    let r = run_cli(
        &["mailboxes", "list", "-o", "table"],
        &env(ENV),
        &h,
        &FakeExec::new(),
    );
    assert_eq!(r.out[0], "address   n \nb@x.org   3 \ncc@x.org  10");
}

#[test]
fn password_manager_references_map_to_the_right_commands() {
    let ex = FakeExec::new();
    assert_eq!(
        resolve_secret("op://V/I/f", &ex, "linux").unwrap(),
        "secret"
    );
    resolve_secret("bw:migadu", &ex, "linux").unwrap();
    resolve_secret("pass:mail/migadu", &ex, "linux").unwrap();
    resolve_secret("keepassxc:/a.kdbx#Migadu", &ex, "linux").unwrap();
    resolve_secret("keychain:migadu", &ex, "macos").unwrap();
    resolve_secret("keychain:migadu", &ex, "linux").unwrap();
    let cmds: Vec<String> = ex.0.borrow().iter().map(|c| c[0].clone()).collect();
    assert_eq!(
        cmds,
        [
            "op",
            "bw",
            "pass",
            "keepassxc-cli",
            "security",
            "secret-tool"
        ]
    );
    assert!(resolve_secret("keychain:m", &ex, "windows")
        .unwrap_err()
        .contains("Windows"));
    assert!(resolve_secret("nope", &ex, "linux")
        .unwrap_err()
        .contains("unknown"));
    assert!(resolve_secret("keepassxc:/a.kdbx", &ex, "linux").is_err());
}

#[test]
fn an_unsupported_secret_reference_is_never_echoed_back() {
    let marker = "SENSITIVE-MARKER-123";
    let err = resolve_secret(marker, &FakeExec::new(), "linux").unwrap_err();
    assert!(err.contains("unknown secret reference") && !err.contains(marker));
    type Case<'a> = (&'a str, Vec<(&'a str, &'a str)>, Vec<&'a str>);
    let cases: [Case; 2] = [
        (
            "MIGADU_API_KEY_REF",
            vec![("MIGADU_EMAIL", "a@x.org"), ("MIGADU_API_KEY_REF", marker)],
            vec!["domains", "list"],
        ),
        (
            "MIGADU_WEB_COOKIE_REF",
            vec![("MIGADU_WEB_COOKIE_REF", marker)],
            vec!["tier"],
        ),
    ];
    for (name, pairs, argv) in cases {
        let r = run_cli(&argv, &env(&pairs), &ok_empty(), &FakeExec::new());
        assert_eq!(r.code, 1, "{name}");
        assert!(
            r.stderr().starts_with("error: unknown secret reference"),
            "{name}: {}",
            r.stderr()
        );
        assert!(!r.stderr().contains(marker), "{name} value leaked");
        assert!(r.out.is_empty());
    }
}

#[test]
fn api_key_reference_is_resolved_through_the_password_manager() {
    let ex = FakeExec::new();
    let h = ok_empty();
    let e = env(&[
        ("MIGADU_EMAIL", "a@x.org"),
        ("MIGADU_API_KEY_REF", "op://V/I/f"),
    ]);
    assert_eq!(run_cli(&["domains", "list"], &e, &h, &ex).code, 0);
    assert_eq!(ex.0.borrow()[0], ["op", "read", "op://V/I/f"]);
    let auth = h.calls.borrow()[0]
        .header("authorization")
        .unwrap()
        .to_string();
    assert_eq!(auth, "Basic YUB4Lm9yZzpzZWNyZXQ=");
}

#[test]
fn tier_reads_the_plan_from_the_admin_panel_with_a_session_cookie_no_api_key_needed() {
    let html = r#"<a href="/billing/subscription">Mini / monthly</a> Active Plan renews November 27, 2026"#;
    let h = Fake::new(move |_| resp(200, html));
    let r = run_cli(
        &["tier"],
        &env(&[("MIGADU_WEB_COOKIE", "s=abc")]),
        &h,
        &FakeExec::new(),
    );
    assert_eq!(r.code, 0, "{}", r.stderr());
    let j = r.json();
    assert_eq!(
        (&j["plan"], &j["billing"], &j["renews"], &j["limits"]["api"]),
        (
            &json!("mini"),
            &json!("monthly"),
            &json!("November 27, 2026"),
            &json!(true)
        )
    );
    assert_eq!(h.calls.borrow()[0].header("cookie"), Some("s=abc"));
}

#[test]
fn expired_web_session_gives_a_clear_error() {
    let h = Fake::new(|_| redirect("/public/login"));
    let r = run_cli(
        &["tier"],
        &env(&[("MIGADU_WEB_COOKIE", "s=x")]),
        &h,
        &FakeExec::new(),
    );
    assert_eq!(r.code, 1);
    assert!(r.stderr().contains("session expired"));
}
