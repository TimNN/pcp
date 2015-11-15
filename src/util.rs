use libc;

pub fn stdin_isatty() -> bool {
    unsafe { libc::isatty(0) != 0 }
}
