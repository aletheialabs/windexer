use rustc_version::{version, Version};
use std::process::Command;

fn main() {
    let version = version().unwrap();
    if version < Version::parse("1.60.0").unwrap() {
        panic!("This crate requires Rust version 1.60.0 or later");
    }
    
    println!("cargo:rustc-env=RUST_VERSION={}", version);
    
    println!("cargo:rustc-env=BUILD_TIMESTAMP={}", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs());
        
    let output = Command::new("git")
        .args(&["rev-parse", "HEAD"])
        .output();
        
    match output {
        Ok(output) if output.status.success() => {
            let git_hash = String::from_utf8_lossy(&output.stdout);
            println!("cargo:rustc-env=GIT_HASH={}", git_hash.trim());
        }
        _ => {
            println!("cargo:rustc-env=GIT_HASH=unknown");
        }
    }
    
    #[cfg(target_arch = "x86_64")]
    {
        println!("cargo:rustc-env=TARGET_ARCH=x86_64");
        
        #[cfg(target_feature = "avx2")]
        {
            println!("cargo:rustc-env=HAS_AVX2=1");
        }
    }
    
    #[cfg(target_arch = "aarch64")]
    {
        println!("cargo:rustc-env=TARGET_ARCH=aarch64");
        
        #[cfg(target_feature = "neon")]
        {
            println!("cargo:rustc-env=HAS_NEON=1");
        }
    }
    
    println!("Building wIndexer Geyser plugin with Rust {}", version);
    
    println!("cargo:rerun-if-changed=build.rs");
    
    println!("cargo:rerun-if-changed=Cargo.toml");
}