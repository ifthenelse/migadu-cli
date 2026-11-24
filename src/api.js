export const BASE_URL = "https://api.migadu.com/v1";

export class MigaduError extends Error {
  constructor(status, body, method, path) {
    super(`${method} ${path} -> HTTP ${status}${body?.error ? `: ${body.error}` : ""}`);
    this.status = status;
    this.body = body;
  }
}

export function createClient({ email, apiKey, base = BASE_URL, fetchImpl = globalThis.fetch }) {
  const auth = "Basic " + Buffer.from(`${email}:${apiKey}`).toString("base64");

  async function raw(method, path, body) {
    const res = await fetchImpl(base + path, {
      method,
      headers: { Authorization: auth, Accept: "application/json", ...(body ? { "Content-Type": "application/json" } : {}) },
      body: body ? JSON.stringify(body) : undefined,
      signal: AbortSignal.timeout(30_000),
    });
    const text = await res.text();
    let data = null;
    try { data = text ? JSON.parse(text) : null; } catch { data = { raw: text }; }
    return { status: res.status, ok: res.ok, data };
  }

  async function request(method, path, body) {
    const r = await raw(method, path, body);
    if (r.ok) return r.data;
    // Migadu sometimes answers a successful DELETE with HTTP 500: confirm by re-reading.
    if (method === "DELETE" && r.status === 500 && (await raw("GET", path)).status === 404) return { deleted: true };
    throw new MigaduError(r.status, r.data, method, path);
  }

  return { request };
}
