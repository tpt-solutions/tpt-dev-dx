//! Fixture binary used by `tpt-cli-snap`'s integration tests.
//!
//! It is intentionally tiny and deterministic: given a subcommand it prints a
//! fixed, snapshot-able output. Not published to crates.io (`publish = false`).

use std::io::{Read, Write};
use std::process::exit;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let sub = args.get(1).map(String::as_str).unwrap_or("hello");

    match sub {
        "hello" => {
            let who = args.get(2).map(String::as_str).unwrap_or("world");
            println!("hello, {who}!");
        }
        "err" => {
            eprintln!("something went wrong");
            exit(1);
        }
        "both" => {
            println!("to stdout");
            eprintln!("to stderr");
        }
        "code" => {
            let code: i32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);
            println!("exiting with code {code}");
            exit(code);
        }
        "env" => {
            let name = args.get(2).map(String::as_str).unwrap_or("CLI_FIXTURE_VAR");
            match std::env::var(name) {
                Ok(v) => println!("env {name}={v}"),
                Err(_) => println!("env {name}=(unset)"),
            }
            // Echo any piped stdin.
            let mut buf = String::new();
            if std::io::stdin().read_to_string(&mut buf).is_ok() && !buf.is_empty() {
                let _ = std::io::stdout().write_all(b"stdin: ");
                let _ = std::io::stdout().write_all(buf.as_bytes());
            }
        }
        other => {
            eprintln!("unknown subcommand: {other}");
            exit(2);
        }
    }
}
