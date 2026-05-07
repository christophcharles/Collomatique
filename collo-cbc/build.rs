fn main() {
    println!("cargo:rerun-if-changed=cpp/collo_cbc.cpp");
    println!("cargo:rerun-if-changed=cpp/collo_cbc.h");

    let cbc = pkg_config::Config::new()
        .atleast_version("2.10")
        .probe("cbc")
        .expect("CBC >= 2.10 required (install libcbc-dev or equivalent)");

    let mut build = cc::Build::new();
    build
        .cpp(true)
        .std("c++17")
        .opt_level(2)
        .file("cpp/collo_cbc.cpp")
        .include("cpp");

    for path in &cbc.include_paths {
        build.include(path);
    }

    build.compile("collo_cbc_shim");

    for path in &cbc.link_paths {
        println!("cargo:rustc-link-search=native={}", path.display());
    }
    for lib in &cbc.libs {
        println!("cargo:rustc-link-lib={}", lib);
    }
    println!("cargo:rustc-link-lib=stdc++");
}
