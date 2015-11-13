#![feature(vec_push_all)]

use std::{cmp, mem};

use self::pair::*;
use self::Leading::*;

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

impl Problem {
    fn solve(&self, max_depth: u32) {
        let mut depth = 0;
        let mut max_set_size = 0;

        let mut current_working_set = Vec::new();
        let mut next_working_set = Vec::new();

        current_working_set.push(VPair::new());

        while depth <= max_depth {
            println!("Now at depth {}", depth);
            max_set_size = cmp::max(max_set_size, current_working_set.len());

            for e in current_working_set.iter() {
                for p in self.pairs.iter() {
                    if let Some(ne) = e.apply(p) {
                        if ne.is_complete() {
                            println!("success!");
                            println!("max set: {}", max_set_size);
                            return;
                        }

                        next_working_set.push(ne);
                    }
                }
            }

            mem::swap(&mut current_working_set, &mut next_working_set);
            next_working_set.clear();

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
