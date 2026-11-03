// SPDX-License-Identifier: MIT
// Copyright (c) 2026 swchen44
#[cfg(target_os = "macos")]
mod host;
#[cfg(target_os = "macos")]
mod mac {
    use rustgrok_path_index::durable::{DurableError, DurablePathIndex};
    use rustgrok_path_index::{CancellationToken, IndexError, IndexLimits};
    use serde_json::{json, Value};
    use std::{
        path::Path,
        sync::{
            atomic::{AtomicBool, Ordering},
            Arc,
        },
        thread,
        time::Duration,
    };

    static INTERRUPTED: AtomicBool = AtomicBool::new(false);
    extern "C" fn on_sigint(_: libc::c_int) {
        INTERRUPTED.store(true, Ordering::Release);
    }
    struct SignalMonitor {
        stop: Arc<AtomicBool>,
        worker: Option<thread::JoinHandle<()>>,
    }
    impl SignalMonitor {
        fn start(token: Arc<CancellationToken>) -> Result<Self, Failure> {
            INTERRUPTED.store(false, Ordering::Release);
            // SAFETY: handler only performs a lock-free atomic store; no borrowed pointers cross the signal boundary.
            if unsafe { libc::signal(libc::SIGINT, on_sigint as *const () as libc::sighandler_t) }
                == libc::SIG_ERR
            {
                return Err(Failure::new(6, "io_error"));
            }
            let stop = Arc::new(AtomicBool::new(false));
            let worker_stop = Arc::clone(&stop);
            let worker = thread::Builder::new()
                .spawn(move || {
                    while !worker_stop.load(Ordering::Acquire) {
                        if INTERRUPTED.load(Ordering::Acquire) {
                            token.cancel();
                            #[cfg(feature = "test-hooks")]
                            if std::env::var_os("RUSTGROK_TEST_PAUSE").as_deref()
                                == Some(std::ffi::OsStr::new("1"))
                            {
                                eprintln!("RUSTGROK_TEST_SIGNAL_OBSERVED");
                            }
                            break;
                        }
                        thread::sleep(Duration::from_millis(10));
                    }
                })
                .map_err(|_| Failure::new(6, "io_error"))?;
            Ok(Self {
                stop,
                worker: Some(worker),
            })
        }
    }
    impl Drop for SignalMonitor {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::Release);
            if let Some(worker) = self.worker.take() {
                let _ = worker.join();
            }
        }
    }
    fn index(
        db: &DurablePathIndex,
        expected: Option<u64>,
        token: &CancellationToken,
    ) -> Result<u64, DurableError> {
        #[cfg(feature = "test-hooks")]
        if std::env::var_os("RUSTGROK_TEST_PAUSE").as_deref() == Some(std::ffi::OsStr::new("1")) {
            use std::io::Read;
            let mut pause = || {
                eprintln!("RUSTGROK_TEST_PAUSED");
                let mut byte = [0u8; 1];
                let _ = std::io::stdin().read_exact(&mut byte);
            };
            return db.replace_from_source_with_precommit_observer(expected, token, &mut pause);
        }
        db.replace_from_source(expected, token)
    }

    pub(crate) const LIMITS: IndexLimits = IndexLimits {
        max_documents: 50_000,
        max_path_bytes: 4096,
        max_total_path_bytes: 16 * 1024 * 1024,
        max_live_generations: 2,
    };
    pub(crate) const MAX_GENERATION: u64 = 1_000_000;
    pub(crate) struct Failure {
        pub(crate) code: i32,
        pub(crate) label: &'static str,
    }
    impl Failure {
        pub(crate) fn new(code: i32, label: &'static str) -> Self {
            Self { code, label }
        }
    }
    pub(crate) fn classify(error: DurableError) -> Failure {
        match error {
            DurableError::Busy | DurableError::Index(IndexError::WriterBusy) => {
                Failure::new(3, "busy")
            }
            DurableError::Index(IndexError::Cancelled) => Failure::new(130, "cancelled"),
            DurableError::PublicationIndeterminate => Failure::new(7, "publication_indeterminate"),
            DurableError::Corrupt => Failure::new(5, "corrupt_store"),
            DurableError::SourceChanged => Failure::new(5, "source_changed"),
            DurableError::Io(_) | DurableError::Index(IndexError::Source(_)) => {
                Failure::new(6, "io_error")
            }
            DurableError::Index(IndexError::NoPublishedGeneration) => {
                Failure::new(4, "no_generation")
            }
            DurableError::Index(IndexError::StaleGeneration { .. }) => {
                Failure::new(4, "stale_generation")
            }
            _ => Failure::new(4, "unsupported_or_limit"),
        }
    }
    fn valid_project(s: &str) -> bool {
        !s.is_empty()
            && s.bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-')
    }
    pub(crate) fn source(s: &str) -> Result<&Path, Failure> {
        let p = Path::new(s);
        if !p.is_absolute() {
            return Err(Failure::new(2, "usage"));
        }
        Ok(p)
    }
    pub(crate) fn project(s: &str) -> Result<&str, Failure> {
        if !valid_project(s) {
            return Err(Failure::new(2, "usage"));
        }
        Ok(s)
    }
    pub(crate) fn open(store: &str, src: &str, name: &str) -> Result<DurablePathIndex, Failure> {
        DurablePathIndex::open(
            source(store)?,
            source(src)?,
            project(name)?,
            LIMITS,
            MAX_GENERATION,
        )
        .map_err(classify)
    }
    fn run(args: &[String]) -> Result<Value, Failure> {
        match args {
            [command, store, src, proj, control] if command == "host" => {
                crate::host::serve(store, src, proj, control)
            }
            [command, control, term] if command == "host-query" => {
                crate::host::client(control, "query", Some(term))
            }
            [command, control, expected] if command == "host-index" => {
                crate::host::client(control, "index", Some(expected))
            }
            [command, control] if command == "host-status" => {
                crate::host::client(control, "status", None)
            }
            [command, control] if command == "host-cancel" => {
                crate::host::client(control, "cancel", None)
            }
            [command, control] if command == "host-stop" => {
                crate::host::client(control, "stop", None)
            }
            #[cfg(feature = "test-hooks")]
            [command, control] if command == "host-test-release" => {
                crate::host::client(control, "test-release", None)
            }
            [command, parent, name, src, proj] if command == "create" => {
                DurablePathIndex::create(
                    source(parent)?,
                    name,
                    source(src)?,
                    project(proj)?,
                    LIMITS,
                    MAX_GENERATION,
                )
                .map_err(classify)?;
                Ok(json!({"operation":"create","status":"created"}))
            }
            [command, store, src, proj, expected] if command == "index" => {
                let expected = if expected == "none" {
                    None
                } else {
                    if expected.is_empty() || !expected.bytes().all(|b| b.is_ascii_digit()) {
                        return Err(Failure::new(2, "usage"));
                    }
                    let n = expected
                        .parse::<u64>()
                        .map_err(|_| Failure::new(2, "usage"))?;
                    if n == 0 {
                        return Err(Failure::new(2, "usage"));
                    }
                    Some(n)
                };
                let token = Arc::new(CancellationToken::new());
                let _signals = SignalMonitor::start(Arc::clone(&token))?;
                let db = open(store, src, proj)?;
                let id = index(&db, expected, &token).map_err(classify)?;
                Ok(json!({"operation":"index","generation":id}))
            }
            [command, store, src, proj, term] if command == "query" => {
                let db = open(store, src, proj)?;
                let snapshot = db.snapshot().map_err(classify)?;
                let paths = snapshot
                    .lookup(term)
                    .map_err(|e| classify(DurableError::Index(e)))?;
                Ok(
                    json!({"operation":"query","generation":snapshot.generation(),"order":"unspecified","paths":paths}),
                )
            }
            _ => Err(Failure::new(2, "usage")),
        }
    }
    pub fn main() -> i32 {
        let args = std::env::args_os()
            .skip(1)
            .map(|arg| arg.into_string())
            .collect::<Result<Vec<_>, _>>();
        match args
            .map_err(|_| Failure::new(2, "usage"))
            .and_then(|v| run(&v))
        {
            Ok(value) => {
                println!("{value}");
                0
            }
            Err(e) => {
                eprintln!("{}", json!({"error":e.label}));
                e.code
            }
        }
    }
}
#[cfg(target_os = "macos")]
fn main() {
    std::process::exit(mac::main())
}
#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("{{\"error\":\"unsupported_platform\"}}");
    std::process::exit(4)
}
