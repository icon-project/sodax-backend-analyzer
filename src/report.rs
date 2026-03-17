use std::fs::{self, File};
use std::io::Write;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

static REPORT_FILE: OnceLock<Mutex<File>> = OnceLock::new();
static REPORT_PATH: OnceLock<String> = OnceLock::new();

pub fn init_report(enabled: bool, args: &[String]) {
  if !enabled {
    return;
  }

  let dir = "reports";
  fs::create_dir_all(dir).expect("Failed to create reports directory");

  let timestamp = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .unwrap()
    .as_secs();

  let filename = format!("{}/report_{}.txt", dir, timestamp);
  let file = File::create(&filename).expect("Failed to create report file");

  REPORT_PATH
    .set(filename.clone())
    .expect("Report path already initialized");
  REPORT_FILE
    .set(Mutex::new(file))
    .expect("Report already initialized");

  // Write command header to report
  let command = args.join(" ");
  write_to_report(&format!("Command: {}", command));
  write_to_report(&format!("Timestamp: {}", timestamp));
  write_to_report("---");

  eprintln!("Report will be saved to: {}", filename);
}

pub fn write_to_report(s: &str) {
  if let Some(file) = REPORT_FILE.get()
    && let Ok(mut f) = file.lock()
  {
    let _ = writeln!(f, "{}", s);
  }
}

pub fn report_path() -> Option<&'static str> {
  REPORT_PATH.get().map(|s| s.as_str())
}

#[macro_export]
macro_rules! output {
  () => {{
    println!();
    $crate::report::write_to_report("");
  }};
  ($($arg:tt)*) => {{
    let s = format!($($arg)*);
    println!("{}", s);
    $crate::report::write_to_report(&s);
  }};
}
