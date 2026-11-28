// Interactive ASCII-art background for the project site: Migadu-flavoured tokens drift, an Alpine
// skyline (Migadu is Swiss) sits at the bottom, characters light up near the pointer, and a click
// sends a little envelope flying. Honours prefers-reduced-motion (one still frame, no pointer effects).
(() => {
  const canvas = document.getElementById("ascii");
  if (!canvas || !canvas.getContext) return;
  const ctx = canvas.getContext("2d");
  const still = matchMedia("(prefers-reduced-motion: reduce)");
  const FONT = 15, LINE = 19, REACH = 130;
  const WORDS = ["@", "MX", "SPF", "DKIM", "DMARC", "IMAP", "SMTP", "POP3", "TLS", "inbox", "alias", "bcc", "catchall", "mailbox", "spam", "CH", "+", "~"];
  const ENVELOPE = [" ________ ", "|\\      /|", "| \\____/ |", "|________|"];
  let w = 0, h = 0, cw = 9, items = [], planes = [], pointer = { x: -1e4, y: -1e4 }, last = 0, raf = 0, ink = {};

  const css = (n, d) => getComputedStyle(document.documentElement).getPropertyValue(n).trim() || d;
  const readInk = () => (ink = { dim: css("--mut", "#64748b"), hot: css("--acc", "#0f766e") });

  function seed() {
    const n = Math.round((w * h) / 22000);
    items = Array.from({ length: n }, (_, i) => {
      const env = i % 7 === 0;
      return { lines: env ? ENVELOPE : [WORDS[(Math.random() * WORDS.length) | 0]], x: Math.random() * w, y: Math.random() * h, vx: (Math.random() - 0.5) * 8, vy: -4 - Math.random() * 8 };
    });
  }

  function resize() {
    const dpr = Math.min(devicePixelRatio || 1, 2);
    w = innerWidth; h = innerHeight;
    canvas.width = w * dpr; canvas.height = h * dpr;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.font = `${FONT}px ui-monospace, SFMono-Regular, Menlo, Consolas, monospace`;
    ctx.textBaseline = "top";
    cw = ctx.measureText("M").width;
    readInk(); seed(); if (still.matches) frame(0);
  }

  const near = (x, y) => Math.hypot(x - pointer.x, y - pointer.y);

  function put(ch, x, y, base) {
    const d = near(x, y), lit = d < REACH ? 1 - d / REACH : 0;
    ctx.globalAlpha = Math.min(1, base + lit * 0.8);
    ctx.fillStyle = lit > 0.05 ? ink.hot : ink.dim;
    ctx.fillText(ch, x, y);
  }

  // Alps: a column-by-column ridge line of / \ ^ and _, drifting slowly sideways.
  function mountains(t) {
    const cols = Math.ceil(w / cw) + 2, base = Math.floor(h / LINE) - 2, shift = t * 0.4;
    const ridge = (c) => Math.round(3.2 * Math.sin((c + shift) * 0.05) + 2.4 * Math.sin((c + shift) * 0.13 + 1) + 1.6 * Math.sin((c + shift) * 0.31) + 6);
    for (let c = 0; c < cols; c++) {
      const r = ridge(c), a = ridge(c - 1), b = ridge(c + 1);
      const ch = r > a && r > b ? "^" : r > a ? "/" : r < a ? "\\" : "_";
      const y = (base - r) * LINE;
      put(ch, c * cw, y, 0.3);
      if (r > 8 && ch === "^") put("*", c * cw, y - LINE, 0.25);
      for (let k = 1; k < 3; k++) put(k % 2 ? "." : ":", c * cw, y + k * LINE, 0.12);
    }
  }

  function frame(now) {
    const dt = Math.min((now - last) / 1000, 0.05); last = now;
    ctx.clearRect(0, 0, w, h);
    for (const it of items) {
      if (!still.matches) {
        const d = near(it.x, it.y);
        if (d < REACH && d > 1) { it.x += ((it.x - pointer.x) / d) * 60 * dt; it.y += ((it.y - pointer.y) / d) * 60 * dt; }
        it.x += it.vx * dt; it.y += it.vy * dt;
        if (it.y < -80) { it.y = h + 20; it.x = Math.random() * w; }
        if (it.x < -120) it.x = w; else if (it.x > w + 20) it.x = -100;
      }
      it.lines.forEach((l, i) => { for (let c = 0; c < l.length; c++) if (l[c] !== " ") put(l[c], it.x + c * cw, it.y + i * LINE, 0.2); });
    }
    mountains(still.matches ? 0 : now / 1000);
    for (const p of planes) {
      p.x += p.vx * dt * 60; p.y += p.vy * dt * 60; p.life -= dt;
      p.trail.push([p.x, p.y]); if (p.trail.length > 14) p.trail.shift();
      p.trail.forEach(([x, y], i) => put(i % 2 ? "." : "-", x, y + 4, 0.08 + (i / 14) * 0.4 * Math.max(p.life, 0)));
      ctx.fillStyle = ink.hot; ctx.globalAlpha = Math.max(p.life, 0); ctx.fillText("[@]>", p.x, p.y);
    }
    planes = planes.filter((p) => p.life > 0);
    ctx.globalAlpha = 1;
    if (!still.matches) raf = requestAnimationFrame(frame);
  }

  function start() { cancelAnimationFrame(raf); last = performance.now(); if (still.matches) frame(0); else raf = requestAnimationFrame(frame); }

  addEventListener("pointermove", (e) => { pointer.x = e.clientX; pointer.y = e.clientY; });
  addEventListener("pointerleave", () => { pointer.x = pointer.y = -1e4; });
  addEventListener("pointerdown", (e) => { if (!still.matches && !e.target.closest("a,button,pre,input")) planes.push({ x: e.clientX, y: e.clientY, vx: 3.2, vy: -1.6, life: 1.6, trail: [] }); });
  addEventListener("resize", resize);
  document.addEventListener("visibilitychange", () => (document.hidden ? cancelAnimationFrame(raf) : start()));
  still.addEventListener("change", start);
  matchMedia("(prefers-color-scheme: dark)").addEventListener("change", () => { readInk(); if (still.matches) frame(0); });
  resize(); start();
})();
