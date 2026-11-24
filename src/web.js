// Read-only access to the admin panel (server-rendered HTML) using a session cookie the user supplies.
// shortcut: HTML regex parsing, breaks if Migadu changes the markup; upgrade to a real parser if it does.
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

  return { page, plan, domains, getCatchall, setCatchall };
}
