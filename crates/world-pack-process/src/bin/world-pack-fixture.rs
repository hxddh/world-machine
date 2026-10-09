//! A scripted stand-in for a Pack process, for this crate's tests on every
//! platform (it replaced `/bin/sh` scripts, which Windows does not have).
//!
//! It reads its script from the file named by the `WORLD_MACHINE_FIXTURE_SCRIPT`
//! setting, one step a line, a tab between a step's verb and its arguments:
//!
//! - `respond <line>`: read one request line (exit 1 at its end), print `<line>`;
//! - `log <path>`: from here on, append every request line read to `<path>`;
//! - `read`: read one request line (exit 1 at its end);
//! - `env <NAME> <path>`: write the value of `NAME` (empty when unset) to `<path>`;
//! - `cwd <path>`: write the working directory to `<path>`;
//! - `stderr <text>`: write `<text>` and a line break to standard error;
//! - `sleep <ms>`: sleep;
//! - `touch <path>`: make an empty file;
//! - `exit <code>`: exit with `<code>` at once;
//! - `marker <path>`: if `<path>` exists, this is a second launch; if not,
//!   make it. Steps written `first\t…` then run only on a first launch and
//!   `second\t…` only on a second.
//!
//! At the end of the script it reads the shutdown request (or the end of its
//! input) and exits 0. It is not a Pack anyone ships.

#![forbid(unsafe_code)]

use std::io::{self, BufRead, Write};
use std::path::Path;
use std::time::Duration;
use std::{env, fs, process, thread};

const SCRIPT_SETTING: &str = "WORLD_MACHINE_FIXTURE_SCRIPT";

fn main() {
    let Some(script) = env::var_os(SCRIPT_SETTING) else {
        eprintln!("world-pack-fixture: {SCRIPT_SETTING} is not set");
        process::exit(2);
    };
    let script = match fs::read_to_string(&script) {
        Ok(script) => script,
        Err(error) => {
            eprintln!("world-pack-fixture: could not read its script: {error}");
            process::exit(2);
        }
    };
    let stdin = io::stdin();
    let mut input = stdin.lock();
    let stdout = io::stdout();
    let mut second = false;
    let mut log: Option<String> = None;
    for line in script.lines() {
        let mut step = line;
        if let Some(rest) = step.strip_prefix("first\t") {
            if second {
                continue;
            }
            step = rest;
        } else if let Some(rest) = step.strip_prefix("second\t") {
            if !second {
                continue;
            }
            step = rest;
        }
        let (verb, rest) = step.split_once('\t').unwrap_or((step, ""));
        match verb {
            "respond" => {
                read_request(&mut input, log.as_deref());
                let mut out = stdout.lock();
                let _ = writeln!(out, "{rest}");
                let _ = out.flush();
            }
            "read" => read_request(&mut input, log.as_deref()),
            "log" => log = Some(rest.to_string()),
            "env" => {
                let (name, path) = rest.split_once('\t').unwrap_or((rest, ""));
                let _ = fs::write(path, env::var(name).unwrap_or_default());
            }
            "cwd" => {
                let here = env::current_dir().unwrap_or_default();
                let _ = fs::write(rest, here.to_string_lossy().as_bytes());
            }
            "stderr" => eprintln!("{rest}"),
            "sleep" => thread::sleep(Duration::from_millis(rest.parse().unwrap_or(0))),
            "touch" => {
                let _ = fs::write(rest, b"");
            }
            "exit" => process::exit(rest.parse().unwrap_or(1)),
            "marker" => {
                second = Path::new(rest).exists();
                if !second {
                    let _ = fs::write(rest, b"");
                }
            }
            "" => {}
            other => {
                eprintln!("world-pack-fixture: unknown step {other}");
                process::exit(2);
            }
        }
    }
    let mut shutdown = String::new();
    let _ = input.read_line(&mut shutdown);
}

fn read_request(input: &mut impl BufRead, log: Option<&str>) {
    let mut line = String::new();
    match input.read_line(&mut line) {
        Ok(0) | Err(_) => process::exit(1),
        Ok(_) => {}
    }
    if let Some(log) = log {
        if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(log) {
            let _ = file.write_all(line.as_bytes());
        }
    }
}
