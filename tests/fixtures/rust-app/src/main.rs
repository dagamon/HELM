use std::env;
use std::io::Write;

fn main() {
    let mode = env::args().nth(1).unwrap_or_else(|| "echo".to_string());
    match mode.as_str() {
        "echo" => {
            println!("hello-from-rust");
            std::io::stdout().flush().ok();
        }
        "fail" => {
            eprintln!("boom");
            std::process::exit(3);
        }
        "sleep" => {
            std::thread::sleep(std::time::Duration::from_secs(60));
        }
        other => {
            eprintln!("unknown mode: {other}");
            std::process::exit(1);
        }
    }
}
