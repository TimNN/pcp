use std::sync::atomic::{AtomicUsize, ATOMIC_USIZE_INIT, Ordering};

use time::{Duration, PreciseTime};

static CHUNK_ALLOC_COUNT: AtomicUsize = ATOMIC_USIZE_INIT;
static CHUNK_DEALLOC_COUNT: AtomicUsize = ATOMIC_USIZE_INIT;

pub fn chunk_allocated() {
    CHUNK_ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
}

pub fn chunk_deallocated() {
    CHUNK_DEALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
}

pub struct IterStats {
    timings: Vec<Duration>,
    iter_cnt: u32,
}

impl IterStats {
    pub fn new() -> IterStats {
        IterStats {
            timings: vec![],
            iter_cnt: 0,
        }
    }

    pub fn iter<F: FnOnce()>(&mut self, f: F) {
        let start = PreciseTime::now();
        self.iter_cnt += 1;

        println!("Now starting iteration {}", self.iter_cnt);

        f();

        let stop = PreciseTime::now();

        self.timings.push(start.to(stop));
    }
}
