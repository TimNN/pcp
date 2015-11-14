#![feature(alloc, drain, heap_api, oom, unique)]
extern crate alloc;

use std::{cmp, mem};

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
        let mut depth = 0;

        let mut current_working_set: Vec<Chunk<State>> = Vec::new();
        let mut next_working_set = Vec::new();
        let mut chunks = ChunkVec::new();

        let mut w = chunks.get().writer();
        let _ = w.push(State{
            pair: VPair::new(),
            sum: 0,
        });

        current_working_set.push(w.into());

        while depth <= max_depth {
            println!("Now at depth {}", depth);

            let mut write = chunks.get().writer();

            for chunk in current_working_set.drain(..) {
                for state in chunk.iter() {
                    for (id, pair) in self.pairs.iter().enumerate() {
                        if let Some(new_pair) = state.pair.apply(pair) {
                            let new_state = State {
                                pair: new_pair,
                                sum: state.sum + id as u32 + 1,
                            };

                            if new_state.pair.is_complete() {
                                println!("success! n: {}, s: {}", depth + 1, new_state.sum);
                                return;
                            }

                            match write.push(new_state) {
                                Ok(()) => (),
                                Err(ns) => {
                                    next_working_set.push(write.into());
                                    write = chunks.get_with(ns).writer();
                                }
                            }
                        }
                    }
                }

                chunks.offer(chunk);
            }

            next_working_set.push(write.into());

            mem::swap(&mut current_working_set, &mut next_working_set);
            assert!(next_working_set.is_empty());

            depth += 1;
        }
        println!("no success");
    }
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
