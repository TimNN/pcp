use std::cmp;
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::str::FromStr;

use num_cpus;

use pair::{blk, VAL_BITS, SPair, SPart};
use util;

pub struct IPair {
    pub pair: SPair,
    pub sum_inc: u16,
    pub depth_inc: u16,
}

pub struct Config {
    pub pairs: Box<[IPair]>,
    pub max_iter: usize,
    pub thread_cnt: u32,
}

impl Config {
    pub fn from_args() -> Config {
        let matches = clap_app!(myapp =>
            (version: env!("CARGO_PKG_VERSION"))
            (author: "Tim Neumann <mail@timnn.me>")
            (about: "A brute-force pcp solution searcher")
            (@arg max_iter: -n +takes_value "The maximum number of iterations to perform")
            (@arg file: -f +takes_value "Read the problem from the specified file instead of stdin")
        ).get_matches();


        let pairs = match matches.value_of("file") {
            Some(file) => Config::pairs_from_file(file),
            None => Config::pairs_from_stdin(),
        };

        let max_iter = matches.value_of("max_iter")
            .map(|i| usize::from_str(i).expect("Invalid value for -n"))
            .unwrap_or(usize::max_value());

        Config {
            pairs: pairs,
            max_iter: max_iter,
            thread_cnt: num_cpus::get() as u32,
        }
    }

    fn pairs_from_stdin() -> Box<[IPair]> {
        if util::stdin_isatty() {
            println!("Now reading problem from stdin.");
        }

        let stdin = io::stdin();
        let lock = stdin.lock();

        let raw = Config::raw_pairs_from_stream(lock);

        if util::stdin_isatty() {
            println!("Done reading problem.");
        }

        Config::pairs_from_raw(raw)
    }

    fn pairs_from_file(path: &str) -> Box<[IPair]> {
        let f = File::open(path).expect("Failed to open input file");
        let buf = BufReader::new(f);

        let raw = Config::raw_pairs_from_stream(buf);

        Config::pairs_from_raw(raw)
    }

    fn raw_pairs_from_stream<R: BufRead>(buf: R) -> Vec<(String, String)> {
        let mut raw_pairs = Vec::new();

        for line in buf.lines().map(|l| l.expect("Error while reading input")) {
            let parts = line.split_whitespace().collect::<Vec<_>>();

            if parts.is_empty() {
                // skip empty lines
                continue;
            }

            if parts.len() != 2 {
                println!("Error: Expected two whitespace seperated fields");
                continue;
            }

            raw_pairs.push((String::from(parts[0]), String::from(parts[1])));
        }

        raw_pairs
    }

    fn pairs_from_raw(raw: Vec<(String, String)>) -> Box<[IPair]> {
        let alphabet = Alphabet::from_raw(&raw);

        println!("Found {} symbols.", alphabet.symbol_map.len());
        println!("Using {} bit(s) to encode each symbol.", alphabet.symbol_width);

        let mut id = 1;
        let mut pairs = Vec::new();

        for &(ref a, ref b) in raw.iter() {
            let pair = SPair::new(alphabet.encode(a), alphabet.encode(b));

            pairs.push(IPair {
                pair: pair,
                sum_inc: id,
                depth_inc: 1,
            });

            id += 1;
        }

        pairs.into_boxed_slice()
    }
}

struct Alphabet {
    symbol_width: u32,
    symbol_map: HashMap<char, blk>,
}

impl Alphabet {
    fn from_raw(raw: &[(String, String)]) -> Alphabet {
        Alphabet::from_iter(raw.iter().flat_map(|&(ref a, ref b)| a.chars().chain(b.chars())))
    }

    fn from_iter<I: Iterator<Item=char>>(iter: I) -> Alphabet {
        let mut next_id: blk = 0;
        let mut map = HashMap::new();

        for c in iter {
            map.entry(c).or_insert_with(|| {
                let id = next_id;
                next_id += 1;
                id
            });
        }

        // We support one symbol but this computation would return 0 for 1 symbol, which is technically correct and
        // might even work, but I don't want to verify all the logic in pair.rs.
        let symbol_width = cmp::max(1, next_id.next_power_of_two().trailing_zeros());

        Alphabet {
            symbol_width: symbol_width,
            symbol_map: map,
        }
    }

    fn encode(&self, inp: &str) -> SPart {
        let mut len: blk = 0;
        let mut val: blk = 0;

        for c in inp.chars().rev() {
            val = (val << self.symbol_width) | self.symbol_map.get(&c).unwrap();
            len += self.symbol_width as blk;
        }

        assert!(len <= VAL_BITS as blk, "Invalid input: word too long");

        SPart::new(len, val)
    }
}
