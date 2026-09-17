use std::{env, path::Path};

fn main() {
    let unrar_dir = Path::new("vendor/unrar");

    println!("cargo:rerun-if-changed={}", unrar_dir.display());

    let mut build = cc::Build::new();

    build
        .cpp(true)
        .std("c++17")
        .include(unrar_dir)
        .define("RARDLL", None)
        .define("_UNIX", None)
        .warnings(false);

    // Official UnRAR makefile:
    // OBJECTS
    let objects = [
        "rar",
        "strlist",
        "strfn",
        "pathfn",
        "smallfn",
        "global",
        "file",
        "filefn",
        "filcreat",
        "archive",
        "arcread",
        "unicode",
        "system",
        "crypt",
        "crc",
        "rawread",
        "encname",
        "resource",
        "match",
        "timefn",
        "rdwrfn",
        "consio",
        "options",
        "errhnd",
        "rarvm",
        "secpassword",
        "rijndael",
        "getbits",
        "sha1",
        "sha256",
        "blake2s",
        "hash",
        "extinfo",
        "extract",
        "volume",
        "list",
        "find",
        "unpack",
        "headers",
        "threadpool",
        "rs16",
        "cmddata",
        "ui",
        "largepage",
    ];

    // Official UnRAR makefile:
    // LIB_OBJ
    let lib_objects = ["filestr", "scantree", "dll", "qopen"];

    for name in objects.iter().chain(lib_objects.iter()) {
        build.file(unrar_dir.join(format!("{name}.cpp")));
    }

    build.file("vendor/unrar_bridge.cpp");
    build.compile("unrar");

    let target = env::var("TARGET").unwrap_or_default();

    if target.contains("apple") {
        println!("cargo:rustc-link-lib=c++");
    }
}
