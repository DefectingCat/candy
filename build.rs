use std::process::Command;

#[allow(unused)]
macro_rules! warn {
    ($($tokens: tt)*) => {
        println!("cargo:warning={}", format!($($tokens)*))
    }
}

/// Set environment varibles for build.rs
/// set_env!(NAME=xfy);
macro_rules! set_env {
    ($($tokens: tt)*) => {
        println!("cargo:rustc-env={}", format!($($tokens)*))
    };
}

fn main() {
    rustc_info();
}

fn rustc_info() {
    let rustc_output = Command::new("rustc")
        .args(["-vV"])
        .output()
        .expect("detect rustc info failed")
        .stdout;
    let info_str = String::from_utf8_lossy(&rustc_output);
    let info_arr = info_str
        .split('\n')
        .filter(|info| !info.is_empty())
        .collect::<Vec<_>>();

    set_env!("RUA_COMPILER={}", info_arr[0]);
}
