use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

static PROFILE: Lazy<Mutex<Profiler>> = Lazy::new(|| Mutex::new(Profiler::new()));

struct Profiler {
    passes: HashMap<&'static str, (u64, Duration)>,
    active: bool,
    stack: Vec<(&'static str, Instant)>,
}

impl Profiler {
    fn new() -> Self {
        Self { passes: HashMap::new(), active: false, stack: Vec::new() }
    }

    fn enable() {
        let mut p = PROFILE.lock().unwrap();
        p.active = true;
    }

    fn begin(pass: &'static str) {
        let mut p = PROFILE.lock().unwrap();
        if !p.active { return; }
        p.stack.push((pass, Instant::now()));
    }

    fn end() {
        let mut p = PROFILE.lock().unwrap();
        if !p.active { return; }
        if let Some((pass, start)) = p.stack.pop() {
            let elapsed = start.elapsed();
            let entry = p.passes.entry(pass).or_insert((0, Duration::ZERO));
            entry.0 += 1;
            entry.1 += elapsed;
        }
    }

    fn report() {
        let p = PROFILE.lock().unwrap();
        if !p.active { return; }
        eprintln!("\n  Optimizer Pass Profile:");
        eprintln!("  ───────────────────────");
        let mut entries: Vec<_> = p.passes.iter().collect();
        entries.sort_by_key(|(_, (_, d))| std::cmp::Reverse(d.as_nanos() as u64));
        for (name, (calls, dur)) in &entries {
            eprintln!(
                "  {:>30}  {:>6} calls  {:>10.1} µs",
                name,
                calls,
                dur.as_secs_f64() * 1_000_000.0
            );
        }
    }
}

pub(crate) fn enable() { Profiler::enable(); }
pub(crate) fn begin(pass: &'static str) { Profiler::begin(pass); }
pub(crate) fn end() { Profiler::end(); }
pub(crate) fn report() { Profiler::report(); }
