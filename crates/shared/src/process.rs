//! Process management utilities

/// Error quit with message
///
/// ## Arguments:
/// * `msg` - result message
pub fn no(msg: &str) -> () {
    println!("\x1b[91m{}\x1b[0m", format!("error:\x1b[0m {msg}"));
    std::process::exit(1);
}

/// Success quit with message
///
/// ## Arguments:
/// * `msg` - result message
pub fn yes(msg: &str) -> () {
    println!("\x1b[92m{}\x1b[0m", format!("success:\x1b[0m {msg}"));
    std::process::exit(0);
}
