#![feature(vec_push_all)]

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

fn main() {
    let p = Problem::from([("010", "0"), ("0", "01"), ("101", "0100")].as_ref());
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
