use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
};

fn generate_mobile_bindings() {
    println!("cargo:rerun-if-changed=src/mobile/wrapper.h");
    let arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    let os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    let bindings = bindgen::Builder::default()
        .header("src/mobile/wrapper.h")
        .clang_arg("-Wno-everything")
        .layout_tests(false)
        .clang_arg(if arch == "aarch64" && os == "ios" {
            // https://github.com/rust-lang/rust-bindgen/issues/1211
            "--target=arm64-apple-ios"
        } else {
            ""
        })
        .clang_arg(if arch == "aarch64" && os == "ios" {
            // sdk path find by `xcrun --sdk iphoneos --show-sdk-path`
            let output = Command::new("xcrun")
                .arg("--sdk")
                .arg("iphoneos")
                .arg("--show-sdk-path")
                .output()
                .expect("failed to execute xcrun");
            let inc_path =
                Path::new(String::from_utf8_lossy(&output.stdout).trim()).join("usr/include");
            format!("-I{}", inc_path.to_str().expect("invalid include path"))
        } else {
            "".to_string()
        })
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("mobile_bindings.rs"))
        .expect("Couldn't write bindings!");
}

fn main() {
    let os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    if os == "ios" || os == "macos" || os == "android" {
        generate_mobile_bindings();
    }

    if env::var("PROTO_GEN").is_ok() {
        // println!("cargo:rerun-if-changed=src/config/internal/config.proto");
        protoc_rust::Codegen::new()
            .out_dir("src/config/internal")
            .inputs(&["src/config/internal/config.proto"])
            .customize(protoc_rust::Customize {
                expose_oneof: Some(true),
                expose_fields: Some(true),
                generate_accessors: Some(false),
                lite_runtime: Some(true),
                ..Default::default()
            })
            .run()
            .expect("protoc");

        // println!("cargo:rerun-if-changed=src/config/geosite.proto");
        protoc_rust::Codegen::new()
            .out_dir("src/config")
            .inputs(&["src/config/geosite.proto"])
            .customize(protoc_rust::Customize {
                expose_oneof: Some(true),
                expose_fields: Some(true),
                generate_accessors: Some(false),
                lite_runtime: Some(true),
                ..Default::default()
            })
            .run()
            .expect("protoc");

        protoc_rust::Codegen::new()
            .out_dir("src/app/outbound")
            .inputs(&["src/app/outbound/selector_cache.proto"])
            .customize(protoc_rust::Customize {
                expose_oneof: Some(true),
                expose_fields: Some(true),
                generate_accessors: Some(false),
                lite_runtime: Some(true),
                ..Default::default()
            })
            .run()
            .expect("protoc");
    }
}
