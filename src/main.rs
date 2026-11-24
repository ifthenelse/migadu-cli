use std::collections::HashMap;
use std::io::Read;

fn main() {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let env: HashMap<String, String> = std::env::vars().collect();
    let http = migadu::http::Ureq::new();
    let mut out = |s: &str| println!("{s}");
    let mut err = |s: &str| eprintln!("{s}");
    let stdin = || {
        let mut s = String::new();
        let _ = std::io::stdin().read_to_string(&mut s);
        s
    };
    let mut io = migadu::cli::Io {
        out: &mut out,
        err: &mut err,
        stdin: &stdin,
    };
    std::process::exit(migadu::cli::run(
        &argv,
        &env,
        &http,
        &migadu::secrets::System,
        &mut io,
    ));
}
