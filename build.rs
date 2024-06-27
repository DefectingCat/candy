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
    set_env!("NAME=xfy");
    warn!("hello world");
    println!("cargo:info=test");
}
