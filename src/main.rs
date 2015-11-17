#![feature(alloc, filling_drop, heap_api, mutex_get_mut, oom, unique)]

#[macro_use]
extern crate clap;

extern crate alloc;
extern crate ctrlc;
extern crate libc;
extern crate num_cpus;
extern crate quickersort;
extern crate scoped_threadpool;
extern crate tabwriter;
extern crate time;

use self::config::Config;
use solve::solve;

mod chunk_vec;
mod config;
mod pair;
mod solve;
mod stats;
mod util;

fn main() {
    let config = Config::from_args();
    println!("Running with {} threads.", config.thread_cnt);
    solve(&config);
}
