fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=SOLANA_VALIDATOR_PID");
    
    #[cfg(target_os = "linux")]
    {
        cc::Build::new()
            .file("src/memory_layout.S")
            .compile("memory_layout");
    }
}
