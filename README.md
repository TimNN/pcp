# pcp, a brute-force pcp solution searcher

A rust implementation of a brute-force pcp solution searcher. A problem is read from standard input or a file specified on the command line (see [Usage](#usage) for more details).

A problem file consists of any number of (utf-8 encoded) lines, each non-empty line representing a pair of words, consisting of two whitespace-separated fields, each of which denotes one word of the pair. 

Words may consist of any number of non-whitespace unicode characters, each unique character will be replaced by a unique fixed-width bit pattern. The total number of bits per word must not exceed 56.

See the `*.pcp` files in the root directory for example problem specifications. The `wiki*.pcp` files were taken from the [german wikipedia](https://de.wikipedia.org/wiki/Postsches_Korrespondenzproblem), `homework.pcp` was taken from an exercise of my theoretical computer science class.

## Usage

``` plain
pcp 0.1.0
Tim Neumann <mail@timnn.me>
A brute-force pcp solution searcher

USAGE:
	pcp [FLAGS] [OPTIONS]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -f <file>            Read the problem from the specified file instead of stdin
    -n <max_iter>        The maximum number of iterations to perform
```

## Example Run

``` plain
pcp < homework.pcp
Found 2 symbols.
Using 1 bit(s) to encode each symbol.
Running with 8 threads.
Now starting iteration 183
Now starting iteration 197
Now starting iteration 207
Now starting iteration 213
success! n: 216, s: 396

-- Statistics --
chunk size:                        64 MB
chunks allocated:                  39
chunks deallocated:                 0
chunks total memory:              2.4 GB
chunks in current working set:     32
pairs applied:                    1.1 billion
pairs applied successfully:     403.4 million
number of iterations:             217
total iteration time:             5.7 seconds
operations:                     196.0 thousand ops/ms
```

## Installation

You need a recent nightly version of rust to compile this crate. You can then install it using `cargo install pcp`. If you install from source make sure to compile with `--release`!