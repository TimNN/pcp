#![feature(alloc, heap_api, mutex_get_mut, oom, unique)]
extern crate alloc;
extern crate num_cpus;
extern crate scoped_threadpool;

use scoped_threadpool::Pool;

use std::{cmp, mem};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use self::chunk_vec::{ChunkVec, Chunk};
use self::pair::*;
use self::Leading::*;

mod chunk_vec;
mod pair;

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

struct Problem {
    pairs: Box<[SPair]>,
}

#[derive(Clone, Copy)]
struct State {
    pair: VPair,
    sum: u32,
}

impl Problem {
    fn solve(&self, max_depth: u32) {
        let thread_cnt = num_cpus::get() as u32;
        println!("Running with {} threads", thread_cnt);

        let mut depth = 0;

        let mut pool = Pool::new(thread_cnt);

        let mut current_working_set: Vec<Chunk<State>> = Vec::new();
        let next_working_set = Vec::new();
        let mut chunks = ChunkVec::new();

        let mut w = chunks.get().writer();
        let _ = w.push(State{
            pair: VPair::new(),
            sum: 0,
        });

        current_working_set.push(w.into());

        let mut current_working_set = Mutex::new(current_working_set);
        let mut next_working_set = Mutex::new(next_working_set);
        let chunks = Mutex::new(chunks);

        let success = AtomicBool::new(false);

        while depth <= max_depth && !success.load(Ordering::Acquire) {
            println!("Now at depth {}", depth);

            pool.scoped(|scope| {
                for _ in 0 .. thread_cnt {
                    scope.execute(|| {
                        let mut write = with_lock(&chunks, |c| c.get().writer());

                        while let Some(chunk) = with_lock(&current_working_set, |cws| cws.pop()) {
                            for state in chunk.iter() {
                                for (id, pair) in self.pairs.iter().enumerate() {
                                    if let Some(new_pair) = state.pair.apply(pair) {
                                        let new_state = State {
                                            pair: new_pair,
                                            sum: state.sum + id as u32 + 1,
                                        };

                                        if new_state.pair.is_complete() {
                                            println!("success! n: {}, s: {}", depth + 1, new_state.sum);
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
}

fn with_lock<T, R, F: FnOnce(&mut T) -> R>(mutex: &Mutex<T>, f: F) -> R {
    f(&mut*mutex.lock().unwrap())
}

fn main() {
    let p = Problem::from([("010", "0"), ("0", "01"), ("101", "0100")].as_ref());
    // let p = Problem::from([("1", "101"), ("10", "00"), ("011", "11")].as_ref());
    // let p = Problem::from([("001", "0"), ("01", "011"), ("01", "101"), ("10", "001")].as_ref());
    p.solve(300);
}

impl<'a> From<&'a [(&'a str, &'a str)]> for Problem {
    fn from(s: &'a [(&'a str, &'a str)]) -> Problem {
        let mut pairs = Vec::with_capacity(s.len());

        for &p in s {
            pairs.push(p.into());
        }

        Problem {
            pairs: pairs.into_boxed_slice(),
        }
    }
}
