use std::thread::JoinHandle;
use std::io::{stdin, stdout};
use anyhow::{anyhow, Result};
use std::io::Write;
use std::process::exit;


/// A driver for interactive console usage.
pub struct Console {}

impl Console {
  /// Start a new thread that will read and block on stdin for interactive use.
  pub fn spawn() -> JoinHandle<()> {
    std::thread::spawn(move || {
      Console{}.main().unwrap()
    })
  }

  fn main(&mut self) -> Result<()> {
    let mut stdout = stdout();
    let stdin = stdin();

    println!("hello!");
    loop {
      print!("casts- ");
      stdout.flush()?;
      let mut input_buffer = String::new();
      match stdin.read_line(&mut input_buffer) {
        Ok(0) => {
          println!("Goodbye!");
          exit(0);
        },
        Ok(bytes) => println!("Read {} bytes", bytes),
        Err(e) => return Err(anyhow!("Unknown input error: {}", e)),
      };
    }
  }
}
