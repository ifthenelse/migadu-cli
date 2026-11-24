// Read-only access to the admin panel (server-rendered HTML) using a session cookie the user supplies.
// shortcut: HTML regex parsing, breaks if Migadu changes the markup; upgrade to a real parser if it does.
import { parseForm, setField, getField } from "./forms.js";

export const PANEL = "https://admin.migadu.com";

// Snapshot of the public plan table (admin.migadu.com/billing/subscription, 2026-11-24).
export const PLAN_LIMITS = {
  maxi: { in_per_day: 10000, out_per_day: 2000, storage_gb: 500, api: true },
  standard: { in_per_day: 3000, out_per_day: 500, storage_gb: 100, api: true },
  mini: { in_per_day: 1000, out_per_day: 100, storage_gb: 30, api: true },
  micro: { in_per_day: 200, out_per_day: 20, storage_gb: 5, api: false },
};

export function createWeb({ cookie, fetchImpl = globalThis.fetch }) {
  async function page(path) {
    const res = await fetchImpl(PANEL + path, { headers: { Cookie: cookie }, redirect: "manual", signal: AbortSignal.timeout(30_000) });
    const loc = res.headers?.get?.("location") ?? "";
    if ((res.status >= 300 && res.status < 400 && /login/.test(loc)) || res.url?.includes("/public/login"))
      throw new Error("web session expired: sign in at admin.migadu.com and copy a fresh cookie");
    if (!res.ok) throw new Error(`GET ${path} -> HTTP ${res.status}`);
    return res.text();
  }

  async function plan() {
    const html = await page("/billing/subscription");
    const m = /href="\/billing\/subscription"[^>]*>\s*([A-Za-z]+)\s*\/\s*([A-Za-z]+)\s*</.exec(html);
    if (!m) throw new Error("could not find the plan in the admin panel header (is the cookie valid?)");
    const name = m[1].toLowerCase();
    const renews = /renews\s+([A-Z][a-z]+ \d{1,2}, \d{4})/.exec(html)?.[1] ?? null;
    return { plan: name, billing: m[2].toLowerCase(), renews, ...(PLAN_LIMITS[name] ? { limits: PLAN_LIMITS[name] } : {}) };
  }

  async function domains() {
    const html = await page("/domains");
    return [...html.matchAll(/href="\/domains\/(\d+)"[^>]*>\s*([a-z0-9.-]+\.[a-z]+)\s*</gi)].map((m) => ({ id: m[1], name: m[2] }));
  }

  const unescape = (t) => t.replace(/&lt;/g, "<").replace(/&gt;/g, ">").replace(/&quot;/g, '"').replace(/&#39;|&#x27;/g, "'").replace(/&amp;/g, "&");

  async function domainId(name) {
    const d = (await domains()).find((x) => x.name === name.toLowerCase());
    if (!d) throw new Error(`domain "${name}" not found in the admin panel`);
    return d.id;
  }

  async function readCatchall(name) {
    const id = await domainId(name);
    const html = await page(`/domains/${id}/catchall`);
    const ta = /<textarea[^>]*name="domain\[catchall_destinations\]"[^>]*>([\s\S]*?)<\/textarea>/.exec(html);
    const csrf = /<input[^>]*name="_csrf_token"[^>]*value="([^"]+)"/.exec(html) ?? /<input[^>]*value="([^"]+)"[^>]*name="_csrf_token"/.exec(html);
    if (!ta || !csrf) throw new Error("catchall form not found (page layout changed or session invalid)");
    return { id, csrf: unescape(csrf[1]), destinations: unescape(ta[1]).split(/[\s,]+/).filter(Boolean) };
  }

  async function getCatchall(name) {
    const { destinations } = await readCatchall(name);
    return { domain: name, destinations };
  }

  async function setCatchall(name, destinations) {
    const { id, csrf, destinations: previous } = await readCatchall(name);
    const body = new URLSearchParams({
      _method: "put", _csrf_token: csrf, template: "catchall",
      origin_path: `/domains/${id}/catchall`, "domain[catchall_destinations]": destinations.join("\n"),
    });
    const res = await fetchImpl(`${PANEL}/domains/${id}`, {
      method: "POST", redirect: "manual", body,
      headers: { Cookie: cookie, "Content-Type": "application/x-www-form-urlencoded", Referer: `${PANEL}/domains/${id}/catchall` },
      signal: AbortSignal.timeout(30_000),
    });
    if (res.status >= 400 || /login/.test(res.headers?.get?.("location") ?? "")) throw new Error(`catchall update failed (HTTP ${res.status})`);
    const after = (await readCatchall(name)).destinations;
    // The panel keeps only the local part and re-attaches it to this domain, so compare by local part.
    const local = (l) => l.map((a) => a.split("@")[0].toLowerCase()).join(",");
    if (local(after) !== local(destinations)) throw new Error(`catchall update not applied; current value is [${after.join(", ")}]`);
    return { domain: name, previous, destinations: after, ...(after.join(",") !== destinations.join(",") && { note: "the panel stores catchall recipients as addresses on this domain" }) };
  }

  // --- generic form writes -------------------------------------------------------------------
  async function post(action, fields, referer) {
    const res = await fetchImpl(PANEL + action, {
      method: "POST", redirect: "manual", body: new URLSearchParams(fields),
      headers: { Cookie: cookie, "Content-Type": "application/x-www-form-urlencoded", Referer: PANEL + referer },
      signal: AbortSignal.timeout(30_000),
    });
    if (res.status >= 400 || /login/.test(res.headers?.get?.("location") ?? "")) throw new Error(`panel rejected the change (HTTP ${res.status})`);
  }

  // Spam settings: cli name -> [form template page, panel field name]
  const SPAM = {
    aggressiveness: ["edit", "domain[aggressiveness_level]"], subject_rewriting: ["edit", "domain[subject_rewriting_enabled]"],
    junk_subject_keyword: ["edit", "domain[junk_subject_keyword_spam]"],
    sender_denylist: ["sender_denylist", "domain[sender_blacklist]"], sender_junklist: ["sender_denylist", "domain[sender_junklist]"],
    sender_allowlist: ["sender_allowlist", "domain[sender_whitelist]"], allowlist_precedence: ["sender_allowlist", "domain[sender_whitelist_precedence]"],
    recipient_denylist: ["recipient_denylist", "domain[recipient_blacklist]"],
  };
  const LIST_KEYS = new Set(["sender_denylist", "sender_junklist", "sender_allowlist", "recipient_denylist"]);
  const tokens = (v) => String(v).split(/[\s,]+/).filter(Boolean);

  async function spamForm(id, template) {
    const form = parseForm(await page(`/domains/${id}/spam/${template}`), (body) => body.includes(`value="${template}"`));
    if (!form) throw new Error(`spam form "${template}" not found (page layout changed or session invalid)`);
    return form;
  }

  async function getSpam(name) {
    const id = await domainId(name);
    const out = { domain: name };
    for (const template of new Set(Object.values(SPAM).map(([t]) => t))) {
      const form = await spamForm(id, template);
      for (const [k, [t, field]] of Object.entries(SPAM)) if (t === template) out[k] = LIST_KEYS.has(k) ? tokens(getField(form, field) ?? "") : getField(form, field);
    }
    return out;
  }

  async function setSpam(name, changes) {
    for (const k of Object.keys(changes)) if (!SPAM[k]) throw new Error(`unknown spam setting "${k}" (use ${Object.keys(SPAM).join(", ")})`);
    const id = await domainId(name);
    const before = await getSpam(name);
    for (const template of new Set(Object.keys(changes).map((k) => SPAM[k][0]))) {
      const form = await spamForm(id, template);
      for (const [k, v] of Object.entries(changes)) if (SPAM[k][0] === template) setField(form, SPAM[k][1], LIST_KEYS.has(k) ? tokens(v).join("\n") : v);
      await post(form.action, form.fields, `/domains/${id}/spam/${template}`);
    }
    const after = await getSpam(name);
    for (const [k, v] of Object.entries(changes)) {
      const want = LIST_KEYS.has(k) ? tokens(v).join(",") : String(v), got = LIST_KEYS.has(k) ? after[k].join(",") : String(after[k]);
      if (want !== got) throw new Error(`spam setting "${k}" not applied; current value is ${JSON.stringify(after[k])}`);
    }
    return { domain: name, previous: Object.fromEntries(Object.keys(changes).map((k) => [k, before[k]])), applied: changes };
  }

  // --- Bcc captures ---------------------------------------------------------------------------
  const strip = (h) => unescape(h.replace(/<[^>]*>/g, " ")).replace(/\s+/g, " ").trim();
  const local = (a) => String(a).split("@")[0];

  async function listBcc(name) {
    const id = await domainId(name);
    const html = (await page(`/domains/${id}/bccs`)).replace(/<!--[\s\S]*?-->/g, "");
    return [...html.matchAll(/<tr\b[^>]*>([\s\S]*?)<\/tr>/gi)].flatMap(([, row]) => {
      const bid = /\/bccs\/(\d+)\/edit/.exec(row)?.[1];
      if (!bid) return [];
      const [address, direction, destination, expires, changed] = [...row.matchAll(/<td\b[^>]*>([\s\S]*?)<\/td>/gi)].map((c) => strip(c[1]));
      return [{ id: bid, address, direction, destination, expires, changed }];
    });
  }

  const BCC = { address: "blind_copy[local_part]", destination: "blind_copy[bcc_local_part]", direction: "blind_copy[direction]" };
  function applyBcc(form, fields) {
    for (const [k, v] of Object.entries(fields)) {
      if (!BCC[k]) throw new Error(`unknown bcc field "${k}" (use address, destination, direction)`);
      if (k === "direction" && !["incoming", "outgoing", "all"].includes(v)) throw new Error("direction must be incoming, outgoing or all");
      setField(form, BCC[k], k === "direction" ? v : local(v));
    }
  }

  async function createBcc(name, fields) {
    if (!fields.address || !fields.destination) throw new Error("bcc create needs -f address=<pattern> -f destination=<address> [-f direction=incoming|outgoing|all]");
    const id = await domainId(name);
    const form = parseForm(await page(`/domains/${id}/bccs/new`), (b) => b.includes("blind_copy["));
    if (!form) throw new Error("bcc form not found (page layout changed or session invalid)");
    applyBcc(form, { direction: "all", ...fields });
    await post(form.action, form.fields, `/domains/${id}/bccs/new`);
    const found = (await listBcc(name)).find((b) => b.address.split("@")[0] === local(fields.address));
    if (!found) throw new Error("bcc capture was not created");
    return found;
  }

  async function findBcc(name, address) {
    const found = (await listBcc(name)).find((b) => b.address === address || b.address.split("@")[0] === local(address));
    if (!found) throw new Error(`no bcc capture "${address}" on ${name}`);
    return found;
  }

  async function updateBcc(name, address, fields) {
    const id = await domainId(name), b = await findBcc(name, address);
    const form = parseForm(await page(`/domains/${id}/bccs/${b.id}/edit`), (body) => body.includes("blind_copy["));
    if (!form) throw new Error("bcc edit form not found");
    applyBcc(form, fields);
    await post(form.action, form.fields, `/domains/${id}/bccs/${b.id}/edit`);
    return { previous: b, current: await findBcc(name, fields.address ?? address) };
  }

  async function deleteBcc(name, address) {
    const id = await domainId(name), b = await findBcc(name, address);
    const form = parseForm(await page(`/domains/${id}/bccs/${b.id}/delete`), (body) => body.includes('name="confirmation"'));
    if (!form) throw new Error("bcc delete form not found");
    setField(form, "confirmation", b.address); // the panel asks for the full captured address
    await post(form.action, form.fields, `/domains/${id}/bccs/${b.id}/delete`);
    if ((await listBcc(name)).some((x) => x.id === b.id)) throw new Error("bcc capture was not deleted");
    return { deleted: b };
  }

  return { page, plan, domains, getCatchall, setCatchall, getSpam, setSpam, listBcc, createBcc, updateBcc, deleteBcc };
}
