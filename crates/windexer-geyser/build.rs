fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src");

    #[cfg(target_arch = "x86_64")]
    {
        if std::is_x86_feature_detected!("avx2") {
            println!("cargo:rustc-cfg=has_avx2");
        }
        if std::is_x86_feature_detected!("avx512f") {
            println!("cargo:rustc-cfg=has_avx512");
        }
        if std::is_x86_feature_detected!("sse4.1") {
            println!("cargo:rustc-cfg=has_sse4_1");
        }
    }

    let build_date = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();
    
    std::fs::write(
        std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).join("build_info.rs"),
        format!(
            r#"
            pub const BUILD_DATE: &str = "{}";
            pub const GIT_HASH: &str = "{}";
            "#,
            build_date,
            option_env!("GIT_HASH").unwrap_or("unknown")
        ),
    ).unwrap();
}