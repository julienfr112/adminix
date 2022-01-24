
use std::process::Command;

fn main() {
    Command::new("python3").arg("gendb.py").output().unwrap();
    println!("cargo:rustc-env=DATABASE_URL=sqlite:adminix_tests/example.db");
}