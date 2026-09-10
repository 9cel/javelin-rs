use std::{collections::BTreeSet, env, fs, path::PathBuf, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=upstream/Makefile");
    println!("cargo:rerun-if-changed=upstream/Javelin");

    let host = env::var("HOST").unwrap();
    let target = env::var("TARGET").unwrap();
    let os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    let arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    assert!(
        matches!(os.as_str(), "linux" | "macos") && matches!(arch.as_str(), "x86_64" | "aarch64"),
        "javelin-sys currently supports x86_64 and aarch64 Linux/macOS targets"
    );

    let upstream = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("upstream");
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let makefile = fs::read_to_string(upstream.join("Makefile")).unwrap();

    // jasm runs on the build host, even when the library targets another platform.
    let host_out = out.join("host");
    fs::create_dir_all(&host_out).unwrap();
    let mut assembler = compiler(&host, &host);
    assembler
        .cargo_metadata(false)
        .include(&upstream)
        .out_dir(&host_out)
        .files(
            sources(&makefile, "JASM_SOURCES")
                .iter()
                .map(|p| upstream.join(p)),
        );
    let objects = assembler.compile_intermediates();
    let jasm = host_out.join("jasm");
    run(assembler
        .get_compiler()
        .to_command()
        .args(objects)
        .arg("-o")
        .arg(&jasm));

    let mut library = compiler(&host, &target);
    library.include(&upstream);
    for source in sources(&makefile, "PATTERN_JASM_SOURCES") {
        let generated = out.join("generated").join(source.trim_end_matches(".jasm"));
        fs::create_dir_all(generated.parent().unwrap()).unwrap();
        run(Command::new(&jasm)
            .arg("-o")
            .arg(&generated)
            .arg("-f")
            .arg(upstream.join(source)));
        library.file(generated);
    }

    let mut files = sources(&makefile, "PATTERN_SOURCES");
    files.extend(sources(&makefile, "ASSEMBLER_SOURCES"));
    library.files(files.iter().map(|p| upstream.join(p)));
    library.compile("JavelinPattern");
    println!("cargo:rustc-link-lib=pthread");
    println!("cargo:include={}", upstream.join("Javelin").display());
}

fn compiler(host: &str, target: &str) -> cc::Build {
    let mut build = cc::Build::new();
    build
        .host(host)
        .target(target)
        .cpp(true)
        .std("c++17")
        .opt_level(3)
        .pic(true)
        .warnings(false)
        .define("JBUILDCONFIG_FINAL", None)
        .define("NDEBUG", None)
        .flag("-fomit-frame-pointer")
        .flag("-Wno-c++11-narrowing");

    // Match upstream's Clang default while honoring cc's CXX overrides.
    let names = [
        format!("CXX_{target}"),
        format!("CXX_{}", target.replace('-', "_")),
        if host == target {
            "HOST_CXX"
        } else {
            "TARGET_CXX"
        }
        .into(),
        "CXX".into(),
    ];
    for name in &names {
        println!("cargo:rerun-if-env-changed={name}");
    }
    if !names.iter().any(|name| env::var_os(name).is_some()) {
        build.compiler("clang++");
    }
    assert!(
        build.get_compiler().is_like_clang(),
        "JavelinPattern requires Clang (clang++)"
    );

    // Upstream uses Apple's ARM64 architecture macros on both platforms.
    if target.starts_with("aarch64") {
        build
            .define("__arm64__", "1")
            .define("__LITTLE_ENDIAN__", "1");
    }
    build
}

fn sources(makefile: &str, variable: &str) -> BTreeSet<String> {
    let start = format!("{variable} = \\");
    let mut lines = makefile.lines().skip_while(|line| *line != start);
    assert!(
        lines.next().is_some(),
        "missing {variable} in upstream/Makefile"
    );
    let mut files = BTreeSet::new();
    for line in lines {
        for word in line.split_whitespace() {
            if word.starts_with("Javelin/")
                && (word.ends_with(".cpp") || word.ends_with(".cpp.jasm"))
            {
                files.insert(word.to_owned());
            }
        }
        if !line.trim_end().ends_with('\\') {
            break;
        }
    }
    assert!(!files.is_empty(), "empty {variable} in upstream/Makefile");
    files
}

fn run(command: &mut Command) {
    let status = command
        .status()
        .unwrap_or_else(|err| panic!("failed to run {command:?}: {err}"));
    assert!(status.success(), "{command:?} exited with {status}");
}
