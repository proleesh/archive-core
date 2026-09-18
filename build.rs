use std::{env, path::Path};

fn main() {
    let unrar_dir = Path::new("vendor/unrar");

    println!("cargo:rerun-if-changed={}", unrar_dir.display());

    let mut build = cc::Build::new();

    build_sevenzip();

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
fn build_sevenzip() {
    let root = "vendor/7zip";

    println!("cargo:rerun-if-changed=vendor/sevenzip_bridge.cpp");

    println!("cargo:rerun-if-changed=include/sevenzip_bridge.h");

    let mut build = cc::Build::new();

    println!("cargo:rerun-if-changed=vendor/7zip");

    build
        .cpp(true)
        .std("c++17")
        .define("_FILE_OFFSET_BITS", "64")
        .define("_LARGEFILE_SOURCE", None)
        .include(root)
        .include(format!("{root}/CPP"))
        .include(format!("{root}/C"))
        .include("include")
        .file("vendor/sevenzip_bridge.cpp");

    // ZIP archive implementation
    for file in [
        "ZipAddCommon.cpp",
        "ZipHandler.cpp",
        "ZipHandlerOut.cpp",
        "ZipIn.cpp",
        "ZipItem.cpp",
        "ZipOut.cpp",
        "ZipUpdate.cpp",
        "ZipRegister.cpp",
    ] {
        build.file(format!("{root}/CPP/7zip/Archive/Zip/{file}"));
    }

    // ZIP codecs referenced by the official ZIP handler.
    for file in [
        "CopyCoder.cpp",
        "DeflateEncoder.cpp",
        "DeflateDecoder.cpp",
        "ImplodeDecoder.cpp",
        "LzfseDecoder.cpp",
        "LzmaDecoder.cpp",
        "LzmaEncoder.cpp",
        "LzOutWindow.cpp",
        "Lzma2Encoder.cpp",
        "PpmdZip.cpp",
        "ShrinkDecoder.cpp",
        "XzDecoder.cpp",
        "XzEncoder.cpp",
        "ZstdDecoder.cpp",
    ] {
        build.file(format!("{root}/CPP/7zip/Compress/{file}"));
    }

    // ZIP encryption implementations referenced by ZipAddCommon / ZipHandler.
    for file in [
        "MyAes.cpp",
        "WzAes.cpp",
        "ZipCrypto.cpp",
        "ZipStrong.cpp",
        "HmacSha1.cpp",
        "Pbkdf2HmacSha1.cpp",
        "RandGen.cpp",
    ] {
        build.file(format!("{root}/CPP/7zip/Crypto/{file}"));
    }
    for file in [
        "CRC.cpp",
        "IntToString.cpp",
        "MyString.cpp",
        "MyVector.cpp",
        "MyWindows.cpp",
        "StringConvert.cpp",
        "StringToInt.cpp",
        "UTFConvert.cpp",
    ] {
        build.file(format!("{root}/CPP/Common/{file}"));
    }

    // Platform abstraction used by 7-Zip on macOS/iOS too.
    for file in [
        "FileDir.cpp",
        "FileFind.cpp",
        "FileIO.cpp",
        "FileName.cpp",
        "PropVariant.cpp",
        "PropVariantConv.cpp",
        "PropVariantUtils.cpp",
        "System.cpp",
        "TimeUtils.cpp",
    ] {
        build.file(format!("{root}/CPP/Windows/{file}"));
    }

    // 7-Zip common infrastructure.
    // Based on 7-Zip's official 7ZIP_COMMON_OBJS list.
    for file in [
        "CreateCoder.cpp",
        "CWrappers.cpp",
        "InBuffer.cpp",
        "InOutTempBuffer.cpp",
        "FilterCoder.cpp",
        "LimitedStreams.cpp",
        "LockedStream.cpp",
        "MethodId.cpp",
        "MethodProps.cpp",
        "OffsetStream.cpp",
        "OutBuffer.cpp",
        "ProgressUtils.cpp",
        "PropId.cpp",
        "StreamObjects.cpp",
        "StreamUtils.cpp",
        "UniqBlocks.cpp",
        "FileStreams.cpp",
    ] {
        build.file(format!("{root}/CPP/7zip/Common/{file}"));
    }

    // Archive common infrastructure.
    // Based on official AR_COMMON_OBJS.
    for file in [
        "CoderMixer2.cpp",
        "DummyOutStream.cpp",
        "FindSignature.cpp",
        "InStreamWithCRC.cpp",
        "ItemNameUtils.cpp",
        "MultiStream.cpp",
        "OutStreamWithCRC.cpp",
        "OutStreamWithSha1.cpp",
        "HandlerOut.cpp",
        "ParseProperties.cpp",
    ] {
        build.file(format!("{root}/CPP/7zip/Archive/Common/{file}"));
    }

    // Multithreading C++ support.
    build.file(format!("{root}/CPP/Windows/Synchronization.cpp"));

    for file in [
        "MemBlocks.cpp",
        "OutMemStream.cpp",
        "ProgressMt.cpp",
        "StreamBinder.cpp",
        "VirtThread.cpp",
    ] {
        build.file(format!("{root}/CPP/7zip/Common/{file}"));
    }

    // Compile all C++ sources first.
    build.compile("arkive_sevenzip_cpp");

    // 7-Zip C runtime / codec implementation.
    // IMPORTANT: compile these as C, not C++.
    let mut c_build = cc::Build::new();

    c_build
        .cpp(false)
        .define("_FILE_OFFSET_BITS", "64")
        .define("_LARGEFILE_SOURCE", None)
        .include(root)
        .include(format!("{root}/C"));

    // Official C_OBJS required by the ZIP handler and its codecs.
    for file in [
        "7zBuf2.c",
        "7zCrc.c",
        "7zCrcOpt.c",
        "7zStream.c",
        "Aes.c",
        "AesOpt.c",
        "Alloc.c",
        "Bcj2.c",
        "Bcj2Enc.c",
        "Blake2s.c",
        "Bra.c",
        "Bra86.c",
        "BraIA64.c",
        "BwtSort.c",
        "CpuArch.c",
        "Delta.c",
        "HuffEnc.c",
        "LzFind.c",
        "LzFindMt.c",
        "LzFindOpt.c",
        "Lzma2Dec.c",
        "Lzma2DecMt.c",
        "Lzma2Enc.c",
        "LzmaDec.c",
        "LzmaEnc.c",
        "Md5.c",
        "MtCoder.c",
        "MtDec.c",
        "Ppmd7.c",
        "Ppmd7Dec.c",
        "Ppmd7aDec.c",
        "Ppmd7Enc.c",
        "Ppmd8.c",
        "Ppmd8Dec.c",
        "Ppmd8Enc.c",
        "Sha1.c",
        "Sha1Opt.c",
        "Sha256.c",
        "Sha256Opt.c",
        "Sha3.c",
        "Sha512.c",
        "Sha512Opt.c",
        "Sort.c",
        "SwapBytes.c",
        "Xxh64.c",
        "Xz.c",
        "XzDec.c",
        "XzEnc.c",
        "XzIn.c",
        "XzCrc64.c",
        "XzCrc64Opt.c",
        "ZstdDec.c",
        "Threads.c",
    ] {
        c_build.file(format!("{root}/C/{file}"));
    }

    c_build.compile("arkive_sevenzip_c");
}
