import { parseArgs } from "node:util";
import { readFileSync } from "node:fs";
import { createClient } from "./api.js";
import { loadCredentials, resolveSecret } from "./secrets.js";
import { createWeb } from "./web.js";

const e = encodeURIComponent;

// resource -> path builder. `scope` = positional args that precede the item id.
const RESOURCES = {
  mailboxes: { path: (d) => `/domains/${e(d)}/mailboxes`, key: "mailboxes" },
  aliases: { path: (d) => `/domains/${e(d)}/aliases`, key: "address_aliases" },
  rewrites: { path: (d) => `/domains/${e(d)}/rewrites`, key: "rewrites" },
  identities: { path: (d, mb) => `/domains/${e(d)}/mailboxes/${e(mb)}/identities`, key: "identities", scope: "mailbox" },
  forwardings: { path: (d, mb) => `/domains/${e(d)}/mailboxes/${e(mb)}/forwardings`, key: "forwardings", scope: "mailbox" },
};
const DOMAIN_ACTIONS = { records: "GET", diagnostics: "GET", usage: "GET", activate: "POST" };

export const HELP = `migadu — Migadu email hosting from your terminal

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
`;

function buildBody(values, stdin) {
  let body = {};
  if (values.json) {
    const src = values.json === "-" ? stdin() : values.json.startsWith("@") ? readFileSync(values.json.slice(1), "utf8") : values.json;
    body = JSON.parse(src);
  }
  for (const f of values.field ?? []) {
    const i = f.indexOf("=");
    if (i < 1) throw new Error(`bad --field "${f}", expected k=v or k:=json`);
    const typed = f[i - 1] === ":";
    const k = f.slice(0, typed ? i - 1 : i), v = f.slice(i + 1);
    body[k] = typed ? JSON.parse(v) : v;
  }
  return Object.keys(body).length ? body : undefined;
}

function table(data) {
  const rows = Array.isArray(data) ? data : Object.values(data).find(Array.isArray);
  if (!rows?.length || typeof rows[0] !== "object") return JSON.stringify(data, null, 2);
  const cols = Object.keys(rows[0]).filter((k) => typeof rows[0][k] !== "object").slice(0, 6);
  const w = cols.map((c) => Math.max(c.length, ...rows.map((r) => String(r[c] ?? "").length)));
  const line = (r) => cols.map((c, i) => String(r[c] ?? "").padEnd(w[i])).join("  ");
  return [line(Object.fromEntries(cols.map((c) => [c, c]))), ...rows.map(line)].join("\n");
}

// Plan names are not documented in the API; surface any plan/tier-looking fields and raw usage.
function findPlanFields(o, path = "", out = {}) {
  for (const [k, v] of Object.entries(o ?? {})) {
    if (v && typeof v === "object") findPlanFields(v, `${path}${k}.`, out);
    else if (/plan|tier/i.test(k)) out[path + k] = v;
  }
  return out;
}

async function tier(client, domain, web) {
  const report = { plan_fields: {}, domains: [], note: "" };
  if (web) Object.assign(report, await web.plan());
  if (!client) return report;
  const list = await client.request("GET", "/domains");
  const domains = list.domains ?? [];
  for (const d of domains.filter((x) => !domain || x.name === domain)) {
    const usage = await client.request("GET", `/domains/${e(d.name)}/usage`).catch((err) => ({ error: err.message }));
    Object.assign(report.plan_fields, findPlanFields(d, `${d.name}.`), findPlanFields(usage, `${d.name}.usage.`));
    report.domains.push({ name: d.name, active: d.active, usage });
  }
  if (!web && !Object.keys(report.plan_fields).length)
    report.note = "The Migadu API returns no plan/tier field. Set MIGADU_WEB_COOKIE (or MIGADU_WEB_COOKIE_REF) to read the plan from the admin panel.";
  return report;
}

export async function main(argv, { env = process.env, fetchImpl, exec, out = (s) => console.log(s), err = (s) => console.error(s), stdin = () => readFileSync(0, "utf8") } = {}) {
  let parsed;
  try {
    parsed = parseArgs({
      args: argv, allowPositionals: true,
      options: {
        domain: { type: "string", short: "d" }, field: { type: "string", short: "f", multiple: true },
        json: { type: "string" }, yes: { type: "boolean", short: "y" },
        output: { type: "string", short: "o", default: "json" }, help: { type: "boolean", short: "h" },
      },
    });
  } catch (x) { err(x.message); return 2; }
  const { values, positionals: [res, action, ...rest] } = parsed;
  if (values.help || !res) { out(HELP); return res || values.help ? 0 : 2; }

  try {
    const cookie = env.MIGADU_WEB_COOKIE || (env.MIGADU_WEB_COOKIE_REF && (await resolveSecret(env.MIGADU_WEB_COOKIE_REF, { exec })));
    const web = (res === "tier" || res === "catchall" || res === "spam" || res === "bcc") && cookie ? createWeb({ cookie, fetchImpl }) : null;
    // `tier` may run on the web cookie alone; everything else needs API credentials.
    const creds = await loadCredentials(env, { exec }).catch((x) => { if (web) return null; throw x; });
    const client = creds ? createClient({ ...creds, fetchImpl }) : null;
    const domain = values.domain ?? env.MIGADU_DOMAIN;
    const needDomain = () => { if (!domain) throw new Error("domain required: use --domain or MIGADU_DOMAIN"); return domain; };
    const body = buildBody(values, stdin);
    let result;

    if (res === "tier") result = await tier(client, domain, web);
    else if (res === "catchall") {
      if (!web) throw new Error("catchall uses the admin panel: set MIGADU_WEB_COOKIE or MIGADU_WEB_COOKIE_REF");
      const name = rest[0] ?? needDomain();
      if (action === "get") result = await web.getCatchall(name);
      else if (action === "set") {
        const list = (body?.destinations ?? "").split(/[\s,]+/).filter(Boolean);
        if (!list.length && !values.yes) throw new Error("refusing to clear the catchall without --yes");
        result = await web.setCatchall(name, list);
      } else throw new Error("usage: migadu catchall get|set [domain] -f destinations=a@x.org,b@x.org");
    } else if (res === "spam") {
      if (!web) throw new Error("spam uses the admin panel: set MIGADU_WEB_COOKIE or MIGADU_WEB_COOKIE_REF");
      const name = rest[0] ?? needDomain();
      if (action === "get") result = await web.getSpam(name);
      else if (action === "set") {
        const changes = {};
        for (const [k, v] of Object.entries(body ?? {})) changes[k] = v;
        if (!Object.keys(changes).length) throw new Error("nothing to set: use -f aggressiveness=0 -f subject_rewriting:=true …");
        result = await web.setSpam(name, changes);
      } else throw new Error("usage: migadu spam get|set [domain] -f key=value");
    } else if (res === "bcc") {
      if (!web) throw new Error("bcc uses the admin panel: set MIGADU_WEB_COOKIE or MIGADU_WEB_COOKIE_REF");
      const name = values.domain ?? env.MIGADU_DOMAIN ?? rest[0];
      if (!name) throw new Error("domain required: use --domain or MIGADU_DOMAIN");
      const arg = values.domain || env.MIGADU_DOMAIN ? rest[0] : rest[1];
      if (action === "list") result = await web.listBcc(name);
      else if (action === "create") result = await web.createBcc(name, body ?? {});
      else if (action === "update" && arg) result = await web.updateBcc(name, arg, body ?? {});
      else if (action === "delete" && arg) {
        if (!values.yes) throw new Error(`refusing to delete bcc capture "${arg}" without --yes`);
        result = await web.deleteBcc(name, arg);
      } else throw new Error("usage: migadu bcc list|create|update <address>|delete <address> -d <domain> [-f address= -f destination= -f direction=]");
    } else if (res === "api") {
      if (!action || !rest[0]) throw new Error("usage: migadu api <METHOD> <path>");
      result = await client.request(action.toUpperCase(), rest[0].startsWith("/") ? rest[0] : "/" + rest[0], body);
    } else if (res === "domains") {
      const name = rest[0];
      if (action === "list") result = await client.request("GET", "/domains");
      else if (action === "create") result = await client.request("POST", "/domains", body);
      else if (!name) throw new Error(`usage: migadu domains ${action} <name>`);
      else if (action === "get") result = await client.request("GET", `/domains/${e(name)}`);
      else if (action === "update") result = await client.request("PUT", `/domains/${e(name)}`, body);
      else if (DOMAIN_ACTIONS[action]) result = await client.request(DOMAIN_ACTIONS[action], `/domains/${e(name)}/${action}`, body);
      else throw new Error(`unknown domains action "${action}"`);
    } else if (RESOURCES[res]) {
      const r = RESOURCES[res];
      const scoped = r.scope ? rest.shift() : undefined;
      if (r.scope && !scoped) throw new Error(`usage: migadu ${res} ${action} <mailbox> [id]`);
      const base = r.path(needDomain(), scoped);
      const id = rest[0];
      const item = () => { if (!id) throw new Error(`usage: migadu ${res} ${action} ${r.scope ? "<mailbox> " : ""}<id>`); return `${base}/${e(id)}`; };
      if (action === "list") result = await client.request("GET", base);
      else if (action === "get") result = await client.request("GET", item());
      else if (action === "create") result = await client.request("POST", base, body);
      else if (action === "update") result = await client.request("PUT", item(), body);
      else if (action === "delete") {
        if (!values.yes) throw new Error(`refusing to delete ${res} "${id}" without --yes`);
        result = await client.request("DELETE", item());
      } else throw new Error(`unknown ${res} action "${action}"`);
    } else throw new Error(`unknown resource "${res}"`);

    out(values.output === "table" ? table(result) : JSON.stringify(result, null, 2));
    return 0;
  } catch (x) {
    err(`error: ${x.message}`);
    return 1;
  }
}
