//! Password-manager references. A reference is resolved by running the vendor's own CLI
//! (argument array, no shell); the secret is the first line of its output.

use crate::api::Result;
use std::collections::HashMap;
use std::process::{Command, Stdio};

pub trait Exec {
    fn exec(&self, cmd: &str, args: &[String]) -> Result<String>;
}

/// Runs a vendor CLI with stdin and stderr attached, so master-password prompts still work.
pub struct System;

impl Exec for System {
    fn exec(&self, cmd: &str, args: &[String]) -> Result<String> {
        let out = Command::new(cmd)
            .args(args)
            .stdin(Stdio::inherit())
            .stderr(Stdio::inherit())
            .stdout(Stdio::piped())
            .output()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    format!("{cmd} not found on PATH")
                } else {
                    e.to_string()
                }
            })?;
        if !out.status.success() {
            return Err(match out.status.code() {
                Some(c) => format!("{cmd} exited with code {c}"),
                None => format!("{cmd} was terminated"),
            });
        }
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    }
}

fn first_line(s: &str) -> String {
    s.split('\n')
        .next()
        .unwrap_or("")
        .trim_end_matches('\r')
        .to_string()
}

fn args(a: &[&str]) -> Vec<String> {
    a.iter().map(|s| s.to_string()).collect()
}

/// Reference syntax -> how to read the secret. `platform` is `std::env::consts::OS`.
pub fn resolve_secret(reference: &str, exec: &dyn Exec, platform: &str) -> Result<String> {
    if reference.starts_with("op://") {
        return Ok(first_line(&exec.exec("op", &args(&["read", reference]))?));
    }
    let (scheme, val) = reference.split_once(':').unwrap_or((reference, ""));
    match scheme {
        "bw" => Ok(first_line(&exec.exec("bw", &args(&["get", "password", val]))?)),
        "pass" => Ok(first_line(&exec.exec("pass", &args(&["show", val]))?)),
        "keepassxc" => {
            let (db, entry) = val.split_once('#').unwrap_or((val, ""));
            if db.is_empty() || entry.is_empty() {
                return Err("keepassxc ref must be keepassxc:<file.kdbx>#<entry>".into());
            }
            Ok(first_line(&exec.exec("keepassxc-cli", &args(&["show", "-s", "-a", "Password", db, entry]))?))
        }
        "keychain" => match platform {
            "macos" | "darwin" => Ok(first_line(&exec.exec("security", &args(&["find-generic-password", "-s", val, "-w"]))?)),
            "linux" => Ok(first_line(&exec.exec("secret-tool", &args(&["lookup", "service", val]))?)),
            _ => Err("keychain: is supported on macOS and Linux only; on Windows use op:, bw: or keepassxc:".into()),
        },
        _ => Err("unknown secret reference scheme (expected op://, bw:, pass:, keepassxc: or keychain:)".into()),
    }
}

pub struct Credentials {
    pub email: String,
    pub api_key: String,
}

fn nonempty<'a>(env: &'a HashMap<String, String>, k: &str) -> Option<&'a str> {
    env.get(k).map(|s| s.as_str()).filter(|s| !s.is_empty())
}

pub fn load_credentials(env: &HashMap<String, String>, exec: &dyn Exec) -> Result<Credentials> {
    let email = nonempty(env, "MIGADU_EMAIL")
        .ok_or("MIGADU_EMAIL is not set")?
        .to_string();
    let mut api_key = nonempty(env, "MIGADU_API_KEY").unwrap_or("").to_string();
    if api_key.is_empty() {
        if let Some(r) = nonempty(env, "MIGADU_API_KEY_REF") {
            api_key = resolve_secret(r, exec, std::env::consts::OS)?;
        }
    }
    if api_key.is_empty() {
        return Err("set MIGADU_API_KEY or MIGADU_API_KEY_REF (op://…, bw:…, pass:…, keepassxc:…, keychain:…)".into());
    }
    Ok(Credentials { email, api_key })
}
