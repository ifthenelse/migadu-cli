import { test } from "node:test";
import assert from "node:assert/strict";
import { main } from "../src/cli.js";
import { resolveSecret } from "../src/secrets.js";

const env = { MIGADU_EMAIL: "a@x.org", MIGADU_API_KEY: "k", MIGADU_DOMAIN: "x.org" };

function harness(responder) {
  const calls = [], lines = [], errs = [];
  const fetchImpl = async (url, init) => {
    calls.push({ url, method: init.method, body: init.body && JSON.parse(init.body), auth: init.headers.Authorization });
    const [status, data] = responder(url, init.method, calls.length);
    return { ok: status < 400, status, text: async () => (data === undefined ? "" : JSON.stringify(data)) };
  };
  const run = (argv, e = env) => main(argv, { env: e, fetchImpl, out: (s) => lines.push(s), err: (s) => errs.push(s), stdin: () => '{"a":1}' });
  return { calls, lines, errs, run };
}

test("mailboxes list uses basic auth and domain path", async () => {
  const h = harness(() => [200, { mailboxes: [{ address: "b@x.org" }] }]);
  assert.equal(await h.run(["mailboxes", "list"]), 0);
  assert.equal(h.calls[0].url, "https://api.migadu.com/v1/domains/x.org/mailboxes");
  assert.equal(h.calls[0].auth, "Basic " + Buffer.from("a@x.org:k").toString("base64"));
});

test("create builds body from --field with typed values", async () => {
  const h = harness(() => [200, {}]);
  await h.run(["mailboxes", "create", "-f", "local_part=bob", "-f", "password=12345678", "-f", "may_send:=true"]);
  assert.deepEqual(h.calls[0].body, { local_part: "bob", password: "12345678", may_send: true });
  assert.equal(h.calls[0].method, "POST");
});

test("scoped resources put the mailbox in the path", async () => {
  const h = harness(() => [200, {}]);
  await h.run(["forwardings", "get", "bob", "c@y.org"]);
  assert.equal(h.calls[0].url, "https://api.migadu.com/v1/domains/x.org/mailboxes/bob/forwardings/c%40y.org");
});

test("delete needs --yes", async () => {
  const h = harness(() => [200, {}]);
  assert.equal(await h.run(["aliases", "delete", "info"]), 1);
  assert.equal(h.calls.length, 0);
  assert.equal(await h.run(["aliases", "delete", "info", "--yes"]), 0);
});

test("delete treats HTTP 500 followed by 404 as success", async () => {
  const h = harness((u, m) => (m === "DELETE" ? [500, {}] : [404, {}]));
  assert.equal(await h.run(["mailboxes", "delete", "bob", "-y"]), 0);
});

test("domain actions map to endpoints", async () => {
  const h = harness(() => [200, {}]);
  await h.run(["domains", "activate", "x.org"]);
  assert.deepEqual([h.calls[0].method, h.calls[0].url], ["POST", "https://api.migadu.com/v1/domains/x.org/activate"]);
});

test("tier reports plan fields when present and a note when absent", async () => {
  const h = harness((u) => (u.endsWith("/domains") ? [200, { domains: [{ name: "x.org", active: true, plan: "standard" }] }] : [200, { mailboxes: 3 }]));
  await h.run(["tier"]);
  assert.equal(JSON.parse(h.lines[0]).plan_fields["x.org.plan"], "standard");
  const h2 = harness((u) => (u.endsWith("/domains") ? [200, { domains: [{ name: "x.org" }] }] : [200, {}]));
  await h2.run(["tier"]);
  assert.match(JSON.parse(h2.lines[0]).note, /no plan\/tier/);
});

test("missing credentials fail with a clear message", async () => {
  const h = harness(() => [200, {}]);
  assert.equal(await h.run(["domains", "list"], {}), 1);
  assert.match(h.errs[0], /MIGADU_EMAIL/);
});

test("password manager references map to the right commands", async () => {
  const seen = [];
  const exec = async (c, a) => (seen.push([c, ...a]), "secret\nignored");
  assert.equal(await resolveSecret("op://V/I/f", { exec }), "secret");
  await resolveSecret("bw:migadu", { exec });
  await resolveSecret("pass:mail/migadu", { exec });
  await resolveSecret("keepassxc:/a.kdbx#Migadu", { exec });
  await resolveSecret("keychain:migadu", { exec, platform: "darwin" });
  await resolveSecret("keychain:migadu", { exec, platform: "linux" });
  assert.deepEqual(seen.map((s) => s[0]), ["op", "bw", "pass", "keepassxc-cli", "security", "secret-tool"]);
  await assert.rejects(resolveSecret("keychain:m", { exec, platform: "win32" }), /Windows/);
  await assert.rejects(resolveSecret("nope", { exec }), /unknown/);
});

test("tier reads the plan from the admin panel with a session cookie, no API key needed", async () => {
  const seen = [];
  const html = '<a href="/billing/subscription">Mini / monthly</a> Active Plan renews November 27, 2026';
  const fetchImpl = async (url, init) => (seen.push([url, init.headers.Cookie]), { ok: true, status: 200, headers: new Map(), text: async () => html });
  const lines = [];
  const code = await main(["tier"], { env: { MIGADU_WEB_COOKIE: "s=abc" }, fetchImpl, out: (s) => lines.push(s), err: (s) => lines.push(s) });
  const r = JSON.parse(lines[0]);
  assert.equal(code, 0);
  assert.deepEqual([r.plan, r.billing, r.renews, r.limits.api], ["mini", "monthly", "November 27, 2026", true]);
  assert.equal(seen[0][1], "s=abc");
});

test("expired web session gives a clear error", async () => {
  const fetchImpl = async () => ({ ok: false, status: 302, headers: new Map([["location", "/public/login"]]), text: async () => "" });
  const errs = [];
  const code = await main(["tier"], { env: { MIGADU_WEB_COOKIE: "s=x" }, fetchImpl, out() {}, err: (s) => errs.push(s) });
  assert.equal(code, 1);
  assert.match(errs[0], /session expired/);
});

function panel({ initial = "", apply = true } = {}) {
  let value = initial;
  const posts = [];
  const form = () => `<form><input type="hidden" name="_csrf_token" value="tok&amp;1"><textarea name="domain[catchall_destinations]">${value}</textarea></form>`;
  const fetchImpl = async (url, init = {}) => {
    const h = new Map();
    if (init.method === "POST") {
      posts.push({ url, body: Object.fromEntries(init.body), cookie: init.headers.Cookie });
      if (apply) value = init.body.get("domain[catchall_destinations]");
      h.set("location", "/domains/9/catchall");
      return { ok: false, status: 302, headers: h, text: async () => "" };
    }
    const html = url.endsWith("/domains") ? '<a href="/domains/9">example.org</a>' : form();
    return { ok: true, status: 200, headers: h, text: async () => html };
  };
  return { fetchImpl, posts };
}
const runWeb = async (argv, p) => {
  const lines = [], errs = [];
  const code = await main(argv, { env: { MIGADU_WEB_COOKIE: "s=1" }, fetchImpl: p.fetchImpl, out: (s) => lines.push(s), err: (s) => errs.push(s) });
  return { code, out: lines.join("\n"), err: errs.join("\n") };
};

test("catchall get lists destinations", async () => {
  const r = await runWeb(["catchall", "get", "example.org"], panel({ initial: "a@x.org\nb@x.org" }));
  assert.deepEqual(JSON.parse(r.out).destinations, ["a@x.org", "b@x.org"]);
});

test("catchall set posts the panel's PUT form with csrf and returns the previous value", async () => {
  const p = panel({ initial: "old@x.org" });
  const r = await runWeb(["catchall", "set", "example.org", "-f", "destinations=a@x.org,b@x.org"], p);
  assert.equal(r.code, 0, r.err);
  assert.deepEqual(p.posts[0].body, { _method: "put", _csrf_token: "tok&1", template: "catchall", origin_path: "/domains/9/catchall", "domain[catchall_destinations]": "a@x.org\nb@x.org" });
  assert.equal(p.posts[0].url, "https://admin.migadu.com/domains/9");
  assert.deepEqual(JSON.parse(r.out).previous, ["old@x.org"]);
});

test("catchall set fails loudly when the panel did not apply it, and clearing needs --yes", async () => {
  assert.match((await runWeb(["catchall", "set", "example.org", "-f", "destinations=a@x.org"], panel({ apply: false }))).err, /not applied/);
  assert.match((await runWeb(["catchall", "set", "example.org"], panel())).err, /--yes/);
});
