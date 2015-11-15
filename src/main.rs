#![feature(alloc, heap_api, mutex_get_mut, oom, unique)]

#[macro_use]
extern crate clap;

extern crate alloc;
extern crate libc;
extern crate num_cpus;
extern crate scoped_threadpool;

use scoped_threadpool::Pool;

use std::{cmp, mem};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use self::chunk_vec::{ChunkVec, Chunk};
use self::config::Config;
use self::pair::*;
use self::Leading::*;

mod chunk_vec;
mod config;
mod pair;
mod util;

#[derive(Copy, Clone)]
enum Leading {
    Top,
    Bot,
}

impl Leading {
    pub fn switched(self) -> Leading {
        match self {
            Top => Bot,
            Bot => Top,
        }
    }
}

#[derive(Clone, Copy)]
struct State {
    pair: VPair,
    sum: u16,
    depth: u16,
}

fn solve(config: &Config) {
    let mut depth = 0;

    let mut pool = Pool::new(config.thread_cnt);

    let mut current_working_set: Vec<Chunk<State>> = Vec::new();
    let next_working_set = Vec::new();
    let mut chunks = ChunkVec::new();

    let mut w = chunks.get().writer();
    let _ = w.push(State{
        pair: VPair::new(),
        sum: 0,
        depth: 0,
    });

    current_working_set.push(w.into());

    let mut current_working_set = Mutex::new(current_working_set);
    let mut next_working_set = Mutex::new(next_working_set);
    let chunks = Mutex::new(chunks);

    let success = AtomicBool::new(false);

    while depth <= config.max_iter && !success.load(Ordering::Acquire) {
        println!("Now at depth {}", depth);

        pool.scoped(|scope| {
            for _ in 0 .. config.thread_cnt {
                scope.execute(|| {
                    let mut write = with_lock(&chunks, |c| c.get().writer());

                    while let Some(chunk) = with_lock(&current_working_set, |cws| cws.pop()) {
                        for state in chunk.iter() {
                            for pair in config.pairs.iter() {
                                if let Some(new_pair) = state.pair.apply(&pair.pair) {
                                    let new_state = State {
                                        pair: new_pair,
                                        sum: state.sum + pair.sum_inc,
                                        depth: state.depth + pair.depth_inc,
                                    };

                                    if new_state.pair.is_complete() {
                                        println!("success! n: {}, s: {}", new_state.depth, new_state.sum);
                                        success.store(true, Ordering::Release)
                                    }

                                    match write.push(new_state) {
                                        Ok(()) => (),
                                        Err(ns) => {
                                            with_lock(&next_working_set, |nws| nws.push(write.into()));
                                            write = with_lock(&chunks, |c| c.get_with(ns).writer());
                                        }
                                    }
                                }
                            }
                        }

                        with_lock(&chunks, |c| c.offer(chunk));
                    }

                    with_lock(&next_working_set, |nws| nws.push(write.into()));
                })
            }
        });

        mem::swap(current_working_set.get_mut().unwrap(), next_working_set.get_mut().unwrap());
        assert!(next_working_set.lock().unwrap().is_empty());

        depth += 1;
    }

    if !success.load(Ordering::Acquire) {
        println!("no success");
    }
}

fn with_lock<T, R, F: FnOnce(&mut T) -> R>(mutex: &Mutex<T>, f: F) -> R {
    f(&mut*mutex.lock().unwrap())
}

fn main() {
    let config = Config::from_args();
    println!("Running with {} threads.", config.thread_cnt);
    solve(&config);
}
