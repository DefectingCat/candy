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
    commit_info();
}

/// Get rustc version info
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

/// Get commit info
/// If failed, set RUA_COMMIT=unknown
fn commit_info() {
    let output = Command::new("git").args(["rev-parse", "HEAD"]).output();
    // get commit info
    let Ok(output) = output else {
        warn!("get commit info failed");
        set_env!("RUA_COMMIT={}", "unknown");
        return;
    };
    // check is dirty
    let dirty = Command::new("git")
        .args(["diff", "--quiet"])
        .output()
        .is_ok();
    let commit = String::from_utf8_lossy(&output.stdout)
        .chars()
        .take(7)
        .collect::<String>();
    let commit = if commit.is_empty() {
        "unknown".to_string()
    } else if dirty {
        format!("{commit}-dirty")
    } else {
        commit
    };
    set_env!("RUA_COMMIT={}", commit);
}
