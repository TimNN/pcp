use std::{cmp, ops};
use std::io::{self, Write};
use std::sync::atomic::{AtomicUsize, ATOMIC_USIZE_INIT, Ordering};

use chunk_vec::{Chunk, CHUNK_MB};
use config::Config;
use solve::Node;
use util::OrderingExt;
use util::PoolExt;

use quickersort;
use scoped_threadpool::Pool;
use tabwriter::TabWriter;
use time::{Duration, PreciseTime};

const PRINT_INTERVAL: i64 = 1; // seconds

static CHUNK_ALLOC_COUNT: AtomicUsize = ATOMIC_USIZE_INIT;
static CHUNK_DEALLOC_COUNT: AtomicUsize = ATOMIC_USIZE_INIT;

static PAIR_APPLY_COUNT: AtomicUsize = ATOMIC_USIZE_INIT;
static PAIR_APPLY_SUCCESS_COUNT: AtomicUsize = ATOMIC_USIZE_INIT;

pub fn chunk_allocated() {
    CHUNK_ALLOC_COUNT.fetch_add(1, Ordering::AcqRel);
}

pub fn chunk_deallocated() {
    CHUNK_DEALLOC_COUNT.fetch_add(1, Ordering::AcqRel);
}

pub fn pairs_applied(cnt: usize) {
    PAIR_APPLY_COUNT.fetch_add(cnt, Ordering::AcqRel);
}

pub fn pairs_successfully_applied(cnt: usize) {
    PAIR_APPLY_SUCCESS_COUNT.fetch_add(cnt, Ordering::AcqRel);
}

pub struct IterStats {
    timings: Vec<Duration>,
    iter_cnt: u32,
    last_print: PreciseTime,
}

impl IterStats {
    pub fn new() -> IterStats {
        IterStats {
            timings: vec![],
            iter_cnt: 0,
            last_print: PreciseTime::now(),
        }
    }

    pub fn iter<F: FnOnce()>(&mut self, f: F) {
        let start = PreciseTime::now();
        self.iter_cnt += 1;

        if self.last_print.to(start) >= Duration::seconds(PRINT_INTERVAL) {
            println!("Now starting iteration {}", self.iter_cnt);
            self.last_print = start;
        }

        f();

        let stop = PreciseTime::now();

        self.timings.push(start.to(stop));
    }

    fn iter_duration(&self) -> Duration {
        self.timings.iter().cloned().fold(Duration::zero(), ops::Add::add)
    }
}

macro_rules! stat {
    ($t:expr, $l:expr) => ((writeln!($t, "{}", $l).unwrap()));
    ($t:expr, $l:expr, $v:expr) => ((writeln!($t, "{}:\t{}", $l, $v).unwrap()));
    ($t:expr, $l:expr, $v:expr, $fmt:tt) => ((writeln!($t, concat!("{}:\t", $fmt), $l, $v).unwrap()));
}

macro_rules! stats {
    ($(($($tt:tt)*))*) => {{
        let stdout = io::stdout();
        let mut t = TabWriter::new(stdout.lock());

        $(
            stat!(t, $($tt)*);
        )*

        t.flush().unwrap();
    }}
}

pub fn print(config: &Config, iter_cnt: usize, chunks: &mut [Chunk<Node>], stats: IterStats) {
    if config.analyze {
        println!("Extended analysis requested.");
        analyze(config, chunks);
    }

    let iter_duration = stats.iter_duration();

    stats! {
        ("")
        ("-- Statistics --")
        ("chunk size", CHUNK_MB, "{:5} MB")
        ("chunks allocated", CHUNK_ALLOC_COUNT.load(Ordering::Acquire), "{:5}")
        ("chunks deallocated", CHUNK_DEALLOC_COUNT.load(Ordering::Acquire), "{:5}")
        ("chunks total memory", CHUNK_MB as f64 * CHUNK_ALLOC_COUNT.load(Ordering::Acquire) as f64 / 1024f64, "{:5.1} GB")

        ("chunks in current working set", chunks.len(), "{:5}")
        ("pairs applied", pretty_high_str(PAIR_APPLY_COUNT.load(Ordering::Acquire) as f64))
        ("pairs applied successfully", pretty_high_str(PAIR_APPLY_SUCCESS_COUNT.load(Ordering::Acquire) as f64))

        ("number of iterations", iter_cnt - 1, "{:5}")
        ("total iteration time", iter_duration.num_milliseconds() as f64 / 1000f64, "{:5.1} seconds")
        ("operations", pretty_high_str(PAIR_APPLY_COUNT.load(Ordering::Acquire) as f64 / iter_duration.num_milliseconds() as f64), "{} ops/ms")
    }
}

fn pretty_high_str(f: f64) -> String {
    let (f, suffix) = pretty_high(f);
    format!("{:5.1}{}", f, suffix)
}

fn pretty_high(mut f: f64) -> (f64, &'static str) {
    for suffix in &["", " thousand", " million", " billion", " trillion"] {
        if f < 1000.0 { return (f, suffix) }
        f /= 1000.0;
    }

    return (f, "e15");
}

fn analyze(config: &Config, chunks: &mut [Chunk<Node>]) {
    let mut pool = Pool::new(config.thread_cnt);

    println!("Now sorting chunks.");

    pool.process_each(chunks.iter_mut(), |chunk| {
        quickersort::sort_by(chunk, &cmp_nodes)
    });

    println!("Sorting done!");
}


fn cmp_nodes(a: &Node, b: &Node) -> cmp::Ordering {
    let a = &a.pair;
    let b = &b.pair;

    cmp::Ordering::Equal
        .then(|| a.leading().cmp(&b.leading()))
        .then(|| a.len().cmp(&b.len()))
}
