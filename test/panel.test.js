import { test } from "node:test";
import assert from "node:assert/strict";
import { main } from "../src/cli.js";
import { parseForm, setField, getField } from "../src/forms.js";

const FORM = `<form action="/domains/9/spam" method="post">
<input type="hidden" name="_method" value="put"><input type="hidden" name="_csrf_token" value="t&amp;k">
<input type="hidden" name="template" value="edit">
<select name="domain[aggressiveness_level]"><option value="3">a</option><option value="0" selected>b</option></select>
<input type="hidden" name="domain[subject_rewriting_enabled]" value="false"><input type="checkbox" name="domain[subject_rewriting_enabled]" value="true" checked>
<!-- <input type="checkbox" name="domain[greylisting_enabled]" value="true" checked> -->
<textarea name="domain[x]">
one
two</textarea><input type="submit" name="go" value="Save"></form>`;

test("parseForm returns what a browser would submit", () => {
  const f = parseForm(FORM, (b) => b.includes('value="edit"'));
  assert.deepEqual(f.fields, [["_method", "put"], ["_csrf_token", "t&k"], ["template", "edit"], ["domain[aggressiveness_level]", "0"],
    ["domain[subject_rewriting_enabled]", "false"], ["domain[subject_rewriting_enabled]", "true"], ["domain[x]", "one\ntwo"]]);
  assert.equal(getField(f, "domain[subject_rewriting_enabled]"), true);
  setField(f, "domain[subject_rewriting_enabled]", false);
  assert.equal(getField(f, "domain[subject_rewriting_enabled]"), false);
  setField(f, "domain[aggressiveness_level]", 3);
  assert.equal(getField(f, "domain[aggressiveness_level]"), "3");
});

// Fake panel with the four spam forms, applying PUTs like the real one.
function spamPanel() {
  const v = { aggr: "0", subj: false, junk: false, deny: "", junklist: "", allow: "", prec: false, rcpt: "" };
  const posts = [];
  const cb = (n, on) => `<input type="hidden" name="${n}" value="false"><input type="checkbox" name="${n}" value="true"${on ? " checked" : ""}>`;
  const pages = {
    edit: () => `<form action="/domains/9/spam"><input type="hidden" name="_method" value="put"><input type="hidden" name="_csrf_token" value="c"><input type="hidden" name="template" value="edit"><select name="domain[aggressiveness_level]">${["3", "0", "-3"].map((o) => `<option value="${o}"${o === v.aggr ? " selected" : ""}>x</option>`).join("")}</select>${cb("domain[subject_rewriting_enabled]", v.subj)}${cb("domain[junk_subject_keyword_spam]", v.junk)}</form>`,
    sender_denylist: () => `<form action="/domains/9/spam"><input type="hidden" name="_method" value="put"><input type="hidden" name="_csrf_token" value="c"><input type="hidden" name="template" value="sender_denylist"><textarea name="domain[sender_blacklist]">${v.deny}</textarea><textarea name="domain[sender_junklist]">${v.junklist}</textarea></form>`,
    sender_allowlist: () => `<form action="/domains/9/spam"><input type="hidden" name="_method" value="put"><input type="hidden" name="_csrf_token" value="c"><input type="hidden" name="template" value="sender_allowlist"><textarea name="domain[sender_whitelist]">${v.allow}</textarea>${cb("domain[sender_whitelist_precedence]", v.prec)}</form>`,
    recipient_denylist: () => `<form action="/domains/9/spam"><input type="hidden" name="_method" value="put"><input type="hidden" name="_csrf_token" value="c"><input type="hidden" name="template" value="recipient_denylist"><textarea name="domain[recipient_blacklist]">${v.rcpt}</textarea></form>`,
  };
  const fetchImpl = async (url, init = {}) => {
    const h = new Map();
    if (init.method === "POST") {
      const b = init.body; posts.push(Object.fromEntries(b));
      const t = b.get("template");
      if (t === "edit") { v.aggr = b.get("domain[aggressiveness_level]"); v.subj = b.getAll("domain[subject_rewriting_enabled]").includes("true"); v.junk = b.getAll("domain[junk_subject_keyword_spam]").includes("true"); }
      if (t === "sender_denylist") { v.deny = b.get("domain[sender_blacklist]"); v.junklist = b.get("domain[sender_junklist]"); }
      if (t === "sender_allowlist") { v.allow = b.get("domain[sender_whitelist]"); v.prec = b.getAll("domain[sender_whitelist_precedence]").includes("true"); }
      if (t === "recipient_denylist") v.rcpt = b.get("domain[recipient_blacklist]");
      h.set("location", "/domains/9/spam");
      return { ok: false, status: 302, headers: h, text: async () => "" };
    }
    const tpl = /spam\/(\w+)$/.exec(url)?.[1];
    return { ok: true, status: 200, headers: h, text: async () => (url.endsWith("/domains") ? '<a href="/domains/9">example.org</a>' : pages[tpl]()) };
  };
  return { fetchImpl, posts, v };
}
const run = async (argv, p) => {
  const lines = [], errs = [];
  const code = await main(argv, { env: { MIGADU_WEB_COOKIE: "s=1" }, fetchImpl: p.fetchImpl, out: (s) => lines.push(s), err: (s) => errs.push(s) });
  return { code, out: lines.join("\n"), err: errs.join("\n") };
};

test("spam get reads all four panel forms", async () => {
  const r = await run(["spam", "get", "example.org"], spamPanel());
  assert.deepEqual(JSON.parse(r.out), { domain: "example.org", aggressiveness: "0", subject_rewriting: false, junk_subject_keyword: false,
    sender_denylist: [], sender_junklist: [], sender_allowlist: [], allowlist_precedence: false, recipient_denylist: [] });
});

test("spam set changes only what was asked and preserves sibling fields in the same form", async () => {
  const p = spamPanel(); p.v.junk = true;
  const r = await run(["spam", "set", "example.org", "-f", "aggressiveness=-3", "-f", "subject_rewriting:=true", "-f", "sender_denylist=a@x.org,b@x.org"], p);
  assert.equal(r.code, 0, r.err);
  assert.equal(p.v.aggr, "-3"); assert.equal(p.v.subj, true); assert.equal(p.v.junk, true);
  assert.equal(p.v.deny, "a@x.org\nb@x.org");
  assert.deepEqual(JSON.parse(r.out).previous, { aggressiveness: "0", subject_rewriting: false, sender_denylist: [] });
});

test("spam set rejects unknown keys and reports changes the panel did not apply", async () => {
  assert.match((await run(["spam", "set", "example.org", "-f", "bogus=1"], spamPanel())).err, /unknown spam setting/);
  const p = spamPanel(); const f = p.fetchImpl;
  p.fetchImpl = async (u, i = {}) => (i.method === "POST" ? { ok: false, status: 302, headers: new Map(), text: async () => "" } : f(u, i));
  assert.match((await run(["spam", "set", "example.org", "-f", "aggressiveness=3"], p)).err, /not applied/);
});

// Fake panel for bcc captures with the markup seen live.
function bccPanel() {
  let rows = [], next = 1;
  const posts = [];
  const list = () => `<table>${rows.map((r) => `<tr><td><a href="/domains/9/bccs/${r.id}/edit">${r.a}@example.org</a></td><td>${r.d}</td><td>${r.t}@example.org</td><td><a href="/domains/9/bccs/${r.id}/expire">Never</a></td><td>02:20</td><td><a href="/domains/9/bccs/${r.id}/edit">Edit</a> | <a href="/domains/9/bccs/${r.id}/delete">Delete...</a></td></tr>`).join("")}</table>`;
  const fetchImpl = async (url, init = {}) => {
    const h = new Map();
    if (init.method === "POST") {
      const b = init.body; posts.push({ url, ...Object.fromEntries(b) });
      if (url.endsWith("/bccs")) rows.push({ id: next++, a: b.get("blind_copy[local_part]"), t: b.get("blind_copy[bcc_local_part]"), d: b.get("blind_copy[direction]") });
      else if (b.get("_method") === "delete") { if (b.get("confirmation") === rows[0].a + "@example.org") rows = []; }
      else { const r = rows[0]; r.a = b.get("blind_copy[local_part]"); r.t = b.get("blind_copy[bcc_local_part]"); r.d = b.get("blind_copy[direction]"); }
      return { ok: false, status: 302, headers: new Map([["location", "/domains/9/bccs"]]), text: async () => "" };
    }
    const html = url.endsWith("/domains") ? '<a href="/domains/9">example.org</a>'
      : url.endsWith("/bccs/new") ? '<form action="/domains/9/bccs" method="post"><input type="hidden" name="_csrf_token" value="c"><input type="hidden" name="blind_copy[tags]" value=""><input name="blind_copy[local_part]"><input name="blind_copy[bcc_local_part]"><select name="blind_copy[direction]"><option value="incoming">i</option><option value="outgoing">o</option><option value="all">a</option></select></form>'
      : url.endsWith("/delete") ? '<form action="/domains/9/bccs/1" method="post"><input type="hidden" name="_method" value="delete"><input type="hidden" name="_csrf_token" value="c"><input type="text" name="confirmation"></form>'
      : url.endsWith("/edit") ? `<form action="/domains/9/bccs/1" method="post"><input type="hidden" name="_method" value="put"><input type="hidden" name="_csrf_token" value="c"><input name="blind_copy[local_part]" value="${rows[0].a}"><input name="blind_copy[bcc_local_part]" value="${rows[0].t}"><select name="blind_copy[direction]"><option value="incoming" selected>i</option><option value="all">a</option></select></form>`
      : list();
    return { ok: true, status: 200, headers: h, text: async () => html };
  };
  return { fetchImpl, posts };
}

test("bcc create, list, update and delete follow the panel's forms", async () => {
  const p = bccPanel();
  let r = await run(["bcc", "create", "-d", "example.org", "-f", "address=audit@example.org", "-f", "destination=archive", "-f", "direction=incoming"], p);
  assert.equal(r.code, 0, r.err);
  assert.deepEqual(p.posts[0], { url: "https://admin.migadu.com/domains/9/bccs", _csrf_token: "c", "blind_copy[tags]": "", "blind_copy[local_part]": "audit", "blind_copy[bcc_local_part]": "archive", "blind_copy[direction]": "incoming" });
  r = await run(["bcc", "list", "-d", "example.org"], p);
  assert.deepEqual(JSON.parse(r.out).map((b) => [b.id, b.address, b.direction, b.destination]), [["1", "audit@example.org", "incoming", "archive@example.org"]]);
  r = await run(["bcc", "update", "audit", "-d", "example.org", "-f", "destination=other"], p);
  assert.equal(r.code, 0, r.err);
  assert.equal(JSON.parse(r.out).current.destination, "other@example.org");
  assert.match((await run(["bcc", "delete", "audit", "-d", "example.org"], p)).err, /--yes/);
  r = await run(["bcc", "delete", "audit", "-d", "example.org", "--yes"], p);
  assert.equal(r.code, 0, r.err);
  assert.deepEqual(JSON.parse((await run(["bcc", "list", "-d", "example.org"], p)).out), []);
});

test("bcc validates direction and requires address and destination", async () => {
  assert.match((await run(["bcc", "create", "-d", "example.org", "-f", "address=a"], bccPanel())).err, /needs -f address/);
  assert.match((await run(["bcc", "create", "-d", "example.org", "-f", "address=a", "-f", "destination=b", "-f", "direction=sideways"], bccPanel())).err, /direction must be/);
});
