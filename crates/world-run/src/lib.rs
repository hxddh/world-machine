//! One way to run another program and wait for it, for every part of
//! World Machine that asks one (a model's program, `curl` with a player's
//! key, the Mac's own model, a helper): with what it is given on standard
//! input, a deadline, and a flag that cancels it. A program still running
//! at its deadline or when cancelled is stopped, and is no answer. Both of
//! its pipes are read while it runs, so a long answer can never stall it.

#![forbid(unsafe_code)]

use std::io::{Read as _, Write as _};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// A finished run of a program.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Ran {
    pub success: bool,
    /// Its exit code, when it exited with one.
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

/// Why a run gave no answer.
#[derive(Debug)]
pub enum NotRun {
    /// It could not be started.
    Spawn(std::io::Error),
    /// It was still running at its deadline, and was stopped.
    TimedOut,
    /// It was cancelled, and was stopped.
    Cancelled,
}

/// A flag another thread sets to stop a run.
#[derive(Clone, Debug, Default)]
pub struct Cancel(Arc<AtomicBool>);

impl Cancel {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

/// What to run.
#[derive(Clone, Debug, Default)]
pub struct Run<'a> {
    pub program: &'a str,
    pub args: &'a [String],
    /// Given on standard input, then closed; nothing at all if `None`.
    pub input: Option<&'a str>,
    pub deadline: Option<Instant>,
    pub cancel: Option<Cancel>,
}

impl Run<'_> {
    /// Runs it to the end, its deadline or its cancelling.
    pub fn run(&self) -> Result<Ran, NotRun> {
        let mut child = Command::new(self.program)
            .args(self.args)
            .stdin(if self.input.is_some() {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(NotRun::Spawn)?;
        if let (Some(input), Some(mut stdin)) = (self.input, child.stdin.take()) {
            let input = input.to_string();
            std::thread::spawn(move || {
                let _ = stdin.write_all(input.as_bytes());
            });
        }
        let read = |pipe: Option<Box<dyn std::io::Read + Send>>| {
            std::thread::spawn(move || {
                let mut text = String::new();
                if let Some(mut pipe) = pipe {
                    let _ = pipe.read_to_string(&mut text);
                }
                text
            })
        };
        let out = read(child.stdout.take().map(|pipe| Box::new(pipe) as _));
        let err = read(child.stderr.take().map(|pipe| Box::new(pipe) as _));
        let stopped = loop {
            match child.try_wait() {
                Ok(Some(status)) => break Ok(status),
                Ok(None) => {}
                Err(_) => break Err(NotRun::TimedOut),
            }
            if self.cancel.as_ref().is_some_and(Cancel::is_cancelled) {
                break Err(NotRun::Cancelled);
            }
            if self
                .deadline
                .is_some_and(|deadline| Instant::now() >= deadline)
            {
                break Err(NotRun::TimedOut);
            }
            std::thread::sleep(Duration::from_millis(10));
        };
        let status = match stopped {
            Ok(status) => status,
            Err(why) => {
                let _ = child.kill();
                let _ = child.wait();
                // Whatever it started may still hold the pipes: its readers
                // are left to finish on their own.
                return Err(why);
            }
        };
        Ok(Ran {
            success: status.success(),
            code: status.code(),
            stdout: out.join().unwrap_or_default(),
            stderr: err.join().unwrap_or_default(),
        })
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    fn sh(script: &str) -> Vec<String> {
        vec!["-c".into(), script.into()]
    }

    #[test]
    fn a_run_reads_what_it_is_given_and_what_it_says() {
        let args = sh("cat; echo done >&2");
        let ran = Run {
            program: "/bin/sh",
            args: &args,
            input: Some("hello"),
            deadline: Some(Instant::now() + Duration::from_secs(5)),
            cancel: None,
        }
        .run()
        .unwrap();
        assert!(ran.success);
        assert_eq!(ran.stdout, "hello");
        assert_eq!(ran.stderr.trim(), "done");
    }

    #[test]
    fn a_run_past_its_deadline_or_cancelled_is_stopped() {
        let args = sh("sleep 5");
        let started = Instant::now();
        let timed_out = Run {
            program: "/bin/sh",
            args: &args,
            deadline: Some(Instant::now() + Duration::from_millis(100)),
            ..Run::default()
        }
        .run();
        assert!(matches!(timed_out, Err(NotRun::TimedOut)));
        assert!(started.elapsed() < Duration::from_secs(2));
        let cancel = Cancel::new();
        let flag = cancel.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(50));
            flag.cancel();
        });
        let cancelled = Run {
            program: "/bin/sh",
            args: &args,
            cancel: Some(cancel),
            ..Run::default()
        }
        .run();
        assert!(matches!(cancelled, Err(NotRun::Cancelled)));
        assert!(matches!(
            Run {
                program: "/no/such/program",
                ..Run::default()
            }
            .run(),
            Err(NotRun::Spawn(_))
        ));
    }
}
