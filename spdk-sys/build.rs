extern crate bindgen;
extern crate cc;

use bindgen::callbacks::{MacroParsingBehavior, ParseCallbacks};
use std::{
    collections::HashSet,
    env,
    path::PathBuf,
    sync::{Arc, RwLock},
};

#[derive(Debug)]
struct MacroCallback {
    macros: Arc<RwLock<HashSet<String>>>,
}

impl ParseCallbacks for MacroCallback {
    fn will_parse_macro(&self, name: &str) -> MacroParsingBehavior {
        self.macros.write().unwrap().insert(name.into());

        if name == "IPPORT_RESERVED" {
            return MacroParsingBehavior::Ignore;
        }

        MacroParsingBehavior::Default
    }
}

fn build_wrapper() {
    cc::Build::new()
        .include("spdk/include")
        .file("logwrapper.c")
        .compile("logwrapper");
    cc::Build::new()
        .include("spdk/include")
        .include(".")
        .file("nvme_helper.c")
        .compile("nvme_helper");
}

fn main() {
    #![allow(unreachable_code)]
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    panic!("spdk-sys crate is only for x86_64 (Nehalem or later) and aarch64 (with crypto) ISAs.");
    #[cfg(not(target_os = "linux"))]
    panic!("spdk-sys crate works only on linux");

    let out_dir = env::var("OUT_DIR").unwrap();
    let out_path = PathBuf::from(&out_dir);

    let mut clang_args = Vec::new();

    if let Ok(spdk_path) = env::var("SPDK_PATH") {
        clang_args.push(format!("-I{}/include/spdk/lib", spdk_path));
        clang_args.push(format!("-I{}/include/spdk/module", spdk_path));
        clang_args.push(format!("-I{}/include/spdk_internal", spdk_path));
    } else {
        clang_args.push("-Ispdk/module".into());
        clang_args.push("-Ispdk/lib".into());
        clang_args.push("-Ispdk/include".into());
        clang_args.push("-Ispdk/include/spdk_internal".into());
    }

    build_wrapper();

    let macros = Arc::new(RwLock::new(HashSet::new()));
    let bindings = bindgen::Builder::default()
        .clang_args(clang_args)
        .header("wrapper.h")
        .rustfmt_bindings(true)
        .allowlist_function("*.aio.*")
        .allowlist_function("*.crypto_disk.*")
        .allowlist_function("*.iscsi.*")
        .allowlist_function("*.lock_lba_range")
        .allowlist_function("*.lvol.*")
        .allowlist_function("*.lvs.*")
        .allowlist_function("*.uring.*")
        .allowlist_function("^iscsi.*")
        .allowlist_function("^spdk.*")
        .allowlist_function("create_malloc_disk")
        .allowlist_function("delete_malloc_disk")
        .allowlist_function("^bdev.*")
        .allowlist_function("^nbd_.*")
        .allowlist_function("^vbdev_.*")
        .allowlist_function("^nvme_cmd_.*")
        .allowlist_function("^nvme_status_.*")
        .allowlist_function("^nvmf_subsystem_find_listener")
        .allowlist_function("^nvmf_subsystem_set_ana_state")
        .allowlist_function("^nvmf_subsystem_set_cntlid_range")
        .allowlist_function("^nvmf_tgt_accept")
        .allowlist_function("^nvme_qpair_.*")
        .allowlist_function("^nvme_ctrlr_.*")
        .blocklist_type("^longfunc")
        .allowlist_var("^NVMF.*")
        .allowlist_var("^SPDK.*")
        .allowlist_var("^spdk.*")
        .trust_clang_mangling(false)
        .opaque_type("^spdk_nvme_sgl_descriptor")
        .opaque_type("^spdk_nvme_ctrlr_data")
        .opaque_type("^spdk_nvme_feat_async_event_configuration.*")
        .opaque_type("^spdk_nvmf_fabric_connect.*")
        .opaque_type("^spdk_nvmf_fabric_prop.*")
        .layout_tests(false)
        .derive_default(true)
        .derive_debug(true)
        .derive_copy(true)
        .prepend_enum_name(false)
        .generate_inline_functions(true)
        .parse_callbacks(Box::new(MacroCallback {
            macros,
        }));

    #[cfg(target_arch = "x86_64")]
    let bindings = bindings.clang_arg("-march=nehalem");

    let bindings = bindings.generate().expect("Unable to generate bindings");

    bindings
        .write_to_file(out_path.join("libspdk.rs"))
        .expect("Couldn't write bindings!");

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    println!("cargo:rustc-link-search={}/spdk", manifest_dir);
    println!("cargo:rustc-link-lib=spdk");
    println!("cargo:rustc-link-lib=aio");
    println!("cargo:rustc-link-lib=iscsi");
    println!("cargo:rustc-link-lib=dl");
    println!("cargo:rustc-link-lib=uuid");
    println!("cargo:rustc-link-lib=numa");
    println!("cargo:rustc-link-lib=crypto");
    println!("cargo:rustc-link-lib=uring");

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=wrapper.h");
    println!("cargo:rerun-if-changed=logwrapper.c");
    println!("cargo:rerun-if-changed=nvme_helper.c");
}
