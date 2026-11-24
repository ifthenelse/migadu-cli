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
