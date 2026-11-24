import { spawn } from "node:child_process";

// Run a vendor CLI without a shell (no injection); stdin/stderr stay attached so master-password prompts work.
export function run(cmd, args) {
  return new Promise((resolve, reject) => {
    const p = spawn(cmd, args, { stdio: ["inherit", "pipe", "inherit"] });
    let out = "";
    p.stdout.on("data", (d) => (out += d));
    p.on("error", (e) => reject(new Error(e.code === "ENOENT" ? `${cmd} not found on PATH` : e.message)));
    p.on("close", (code) => (code === 0 ? resolve(out) : reject(new Error(`${cmd} exited with code ${code}`))));
  });
}

const firstLine = (s) => s.split(/\r?\n/)[0];

// Reference syntax -> how to read the secret. Ref examples are in the README.
export async function resolveSecret(ref, { exec = run, platform = process.platform } = {}) {
  const [scheme, ...rest] = ref.split(":");
  const val = rest.join(":");
  if (ref.startsWith("op://")) return firstLine(await exec("op", ["read", ref]));
  switch (scheme) {
    case "bw": return firstLine(await exec("bw", ["get", "password", val]));
    case "pass": return firstLine(await exec("pass", ["show", val]));
    case "keepassxc": {
      const [db, entry] = val.split("#");
      if (!db || !entry) throw new Error("keepassxc ref must be keepassxc:<file.kdbx>#<entry>");
      return firstLine(await exec("keepassxc-cli", ["show", "-s", "-a", "Password", db, entry]));
    }
    case "keychain":
      if (platform === "darwin") return firstLine(await exec("security", ["find-generic-password", "-s", val, "-w"]));
      if (platform === "linux") return firstLine(await exec("secret-tool", ["lookup", "service", val]));
      throw new Error("keychain: is supported on macOS and Linux only; on Windows use op:, bw: or keepassxc:");
    default: throw new Error(`unknown secret reference "${ref}"`);
  }
}

export async function loadCredentials(env, opts) {
  const email = env.MIGADU_EMAIL;
  if (!email) throw new Error("MIGADU_EMAIL is not set");
  const ref = env.MIGADU_API_KEY_REF;
  const apiKey = env.MIGADU_API_KEY || (ref && (await resolveSecret(ref, opts)));
  if (!apiKey) throw new Error("set MIGADU_API_KEY or MIGADU_API_KEY_REF (op://…, bw:…, pass:…, keepassxc:…, keychain:…)");
  return { email, apiKey };
}
