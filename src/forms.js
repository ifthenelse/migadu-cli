// Tiny HTML form reader: returns what a browser would submit for a form, so writes can resend untouched fields.
// shortcut: regex parsing of Migadu's markup, not a general HTML parser; swap for one if the panel changes.
const unescape = (t) => t.replace(/&lt;/g, "<").replace(/&gt;/g, ">").replace(/&quot;/g, '"').replace(/&#39;|&#x27;/g, "'").replace(/&amp;/g, "&");
const attr = (tag, n) => { const m = new RegExp(`\\b${n}\\s*=\\s*(?:"([^"]*)"|'([^']*)')`, "i").exec(tag); return m ? unescape(m[1] ?? m[2]) : undefined; };

export function parseForm(html, pick) {
  html = html.replace(/<!--[\s\S]*?-->/g, ""); // commented-out controls are not part of the form
  const found = [...html.matchAll(/<form\b([^>]*)>([\s\S]*?)<\/form>/gi)].map((m) => ({ tag: m[1], body: m[2] })).find((f) => pick(f.body, f.tag));
  if (!found) return null;
  const fields = [], checkboxes = new Set();
  for (const m of found.body.matchAll(/<(input|select|textarea)\b([^>]*)>(?:([\s\S]*?)<\/\1>)?/gi)) {
    const [, kind, tag, inner = ""] = m, name = attr(tag, "name");
    if (!name || /\bdisabled\b/i.test(tag)) continue;
    if (kind.toLowerCase() === "textarea") fields.push([name, unescape(inner).replace(/^\r?\n/, "")]);
    else if (kind.toLowerCase() === "select") {
      const opts = [...inner.matchAll(/<option\b([^>]*)>/gi)].map((o) => ({ v: attr(o[1], "value") ?? "", sel: /\bselected\b/i.test(o[1]) }));
      const o = opts.find((x) => x.sel) ?? opts[0];
      if (o) fields.push([name, o.v]);
    } else {
      const type = (attr(tag, "type") ?? "text").toLowerCase();
      if (["submit", "button", "file", "image", "reset"].includes(type)) continue;
      if (type === "checkbox") checkboxes.add(name);
      if ((type === "checkbox" || type === "radio") && !/\bchecked\b/i.test(tag)) continue;
      fields.push([name, attr(tag, "value") ?? (type === "checkbox" || type === "radio" ? "on" : "")]);
    }
  }
  return { action: attr(found.tag, "action"), fields, checkboxes };
}

// Replace a field's value; checkboxes follow Rails' hidden "false" + checked "true" pair.
export function setField(form, name, value) {
  const bool = form.checkboxes.has(name);
  form.fields = form.fields.filter(([n]) => n !== name);
  if (bool) { form.fields.push([name, "false"]); if (value === true || value === "true") form.fields.push([name, "true"]); }
  else form.fields.push([name, String(value)]);
}

export const getField = (form, name) => {
  const vals = form.fields.filter(([n]) => n === name).map(([, v]) => v);
  return form.checkboxes.has(name) ? vals.includes("true") : vals.at(-1);
};
