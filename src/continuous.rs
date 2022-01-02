use prost::Message;
use std::io::Write;
use std::path::Path;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use libc::c_int;

use crate::error::Result;
use crate::{ProfilerGuard, ProfilerGuardBuilder};

pub struct ContinuousProfilerGuardBuilder<P: AsRef<Path>> {
    guard_builder: ProfilerGuardBuilder,

    base_dir: P,
    prefix: String,
    rotate_interval: Duration,
}

impl<P: AsRef<Path>> ContinuousProfilerGuardBuilder<P> {
    pub fn new(base_dir: P) -> Self {
        Self {
            guard_builder: ProfilerGuardBuilder::default(),
            base_dir,
            prefix: "profile".into(),
            rotate_interval: Duration::from_secs(60),
        }
    }

    #[must_use = "Builder makes no efforts unless call .build() on it"]
    pub fn frequency(self, frequency: c_int) -> Self {
        Self {
            guard_builder: self.guard_builder.frequency(frequency),
            ..self
        }
    }

    #[cfg(all(any(target_arch = "x86_64", target_arch = "aarch64")))]
    #[must_use = "Builder makes no efforts unless call .build() on it"]
    pub fn blocklist<T: AsRef<str>>(self, blocklist: &[T]) -> Self {
        Self {
            guard_builder: self.guard_builder.blocklist(blocklist),
            ..self
        }
    }

    #[must_use = "Builder makes no efforts unless call .build() on it"]
    pub fn base_dir(self, base_dir: P) -> Self {
        Self { base_dir, ..self }
    }

    #[must_use = "Builder makes no efforts unless call .build() on it"]
    pub fn prefix<S: AsRef<String>>(self, prefix: S) -> Self {
        Self {
            prefix: prefix.as_ref().to_owned(),
            ..self
        }
    }

    #[must_use = "Builder makes no efforts unless call .build() on it"]
    pub fn rotate_interval(self, rotate_interval: Duration) -> Self {
        Self {
            rotate_interval,
            ..self
        }
    }

    pub fn build(self) -> Result<ContinuousProfilerGuard<P>> {
        let guard = self.guard_builder.build()?;

        Ok(ContinuousProfilerGuard {
            guard,
            base_dir: self.base_dir,
            prefix: self.prefix,
            rotate_interval: self.rotate_interval,
        })
    }
}

// todo: make a way to specify the maximum number of files to keep
pub struct ContinuousProfilerGuard<P: AsRef<Path>> {
    guard: ProfilerGuard<'static>,

    base_dir: P,
    prefix: String,
    rotate_interval: Duration,
}

impl<P: AsRef<Path> + Send + 'static> ContinuousProfilerGuard<P> {
    pub fn new(frequency: c_int, base_dir: P) -> Result<ContinuousProfilerGuard<P>> {
        ContinuousProfilerGuardBuilder::new(base_dir)
            .frequency(frequency)
            .build()
    }

    pub fn start(self) -> Result<GuardJoinHandle> {
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        let handle = std::thread::Builder::new()
            .name("profiler".into())
            .spawn(move || {
                while !stop.load(Ordering::Relaxed) {
                    std::thread::sleep(self.rotate_interval);

                    let write_result = || -> Result<()> {
                        let report = self.guard.report().build()?;
                        let now = Instant::now();
                        let path = self
                            .base_dir
                            .as_ref()
                            .join(format!("perf-{}-{:?}.pb", self.prefix, now));
                        let mut file = std::fs::File::create(path)?;

                        let profile = report.pprof().unwrap();
                        let mut content = Vec::new();
                        profile.encode(&mut content).unwrap();
                        file.write_all(&content).unwrap();

                        Ok(())
                    };

                    match write_result() {
                        Ok(_) => {}
                        Err(e) => log::error!("error while writing perf result: {:?}", e),
                    }

                    match self.guard.reset() {
                        Ok(_) => {}
                        Err(e) => log::error!("error while resetting profiler: {:?}", e),
                    }
                }
            })?;

        Ok(GuardJoinHandle { handle, flag })
    }
}

pub struct GuardJoinHandle {
    handle: JoinHandle<()>,
    flag: Arc<AtomicBool>,
}

impl GuardJoinHandle {
    pub fn join(self) -> Result<()> {
        self.flag.store(true, Ordering::Relaxed);
        match self.handle.join() {
            Ok(_) => {}
            Err(e) => log::error!("error while joining profiler thread: {:?}", e),
        }

        Ok(())
    }
}
