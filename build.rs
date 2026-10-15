/// Checks if the target
#[allow(unused)]
fn is_windows_target() -> bool {
    std::env::var("CARGO_CFG_TARGET_OS").map_or(cfg!(windows), |os| os == "windows")
}

pub fn main() {
    /*
     * Rerun if the buildscript, the shim or perl changed.
     */
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/backend/perl_custom_shim.c");
    println!("cargo:rerun-if-env-changed=PERL");

    #[cfg(feature = "perl_custom")]
    perl_custom();
}

#[cfg(feature = "perl_custom")]
fn perl_custom() {
    use perl_custom::EmbedOptions::*;
    use perl_custom::*;

    let perl = Perl::from_env();
    println!(
        "cargo:rerun-if-changed={}",
        std::path::Path::new(&perl.config("archlibexp"))
            .join("Config.pm")
            .display()
    );

    // 1. Shim, compiled with perl's own flags (struct layout must match the perl build).
    let mut build = cc::Build::new();
    build.file("src/backend/perl_custom_shim.c");
    // fallback compiler
    if std::env::var_os("CC").is_none() {
        build.compiler(perl.config("cc")); // the compiler perl was built with
    }
    for arg in split_args(&perl.embed(CCopts)) {
        if let Some(dir) = arg.strip_prefix("-I") {
            // Process includes
            build.include(dir);
        } else if let Some(def) = arg.strip_prefix("-D") {
            // Defines
            let (name, value) = def
                .split_once('=')
                .map_or((def, None), |(n, v)| (n, Some(v)));
            build.define(name, value);
        } else {
            // Pass on everyting else
            build.flag(&arg);
        }
    }
    // Must come before libperl, so the linker resolves the shim's perl references.
    build.compile("nacre_shim");

    // 2. Link libperl the way perl's embedding options say.
    for arg in split_args(&perl.embed(LDopts)) {
        if let Some(dir) = arg.strip_prefix("-L") {
            println!("cargo:rustc-link-search=native={dir}");
        } else if let Some(lib) = arg.strip_prefix("-l") {
            println!("cargo:rustc-link-lib={lib}");
        } else if arg.starts_with("-Wl,") {
            // e.g. -Wl,-E on Linux: exports perl's symbols for XS modules (static libperl).
            println!("cargo:rustc-link-arg={arg}");
        } else if let Some((dir, name)) = lib_file(&arg) {
            // Win32: libperl is given as a bare file path instead of -lperl540.
            println!("cargo:rustc-link-search=native={dir}");
            println!("cargo:rustc-link-lib={name}");
        }
        // Everything else (-s, -fstack-protector-strong, ...) is left to rustc's profile.
    }

    // 3. Perl's integer typedefs for the backend (U32 is unsigned long on Win32, unsigned int on Linux).
    let out = std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).join("perl_types.rs");
    std::fs::write(
        &out,
        format!(
            "pub type U8 = {};\npub type U32 = {};\n",
            c_type(&perl.config("u8type")),
            c_type(&perl.config("u32type")),
        ),
    )
    .unwrap();
}

#[cfg(feature = "perl_custom")]
mod perl_custom {
    use std::ops::Deref;

    /// Wrapper that just holds the path for the perl executable
    pub struct Perl(String);

    pub enum EmbedOptions {
        CCopts,
        LDopts,
    }
    impl Deref for EmbedOptions {
        type Target = str;
        fn deref(&self) -> &Self::Target {
            match self {
                Self::CCopts => "ccopts",
                Self::LDopts => "ldopts",
            }
        }
    }

    impl Perl {
        /// Reads the PERL env.
        /// If that does not exist, is empty or contains only whitespace, then "perl" is used.
        pub fn from_env() -> Self {
            Self(
                std::env::var("PERL")
                    .ok() // error not needed
                    .filter(|p| !p.trim().is_empty()) // only keep if it contains content
                    .unwrap_or_else(|| "perl".to_string()),
            )
        }

        pub fn run(&self, args: &[&str]) -> String {
            let out = std::process::Command::new(&self.0)
                .args(args)
                .output()
                .unwrap_or_else(|e| panic!("Cannot run perl: {e}\nSet env PERL=/path/to/perl"));
            assert!(
                out.status.success(),
                "`{} {args:?}` failed : {}",
                self.0,
                String::from_utf8_lossy(&out.stderr)
            );

            String::from_utf8(out.stdout).expect("Perl did not return UTF8.")
        }

        /// Executes `print $Config{$key}`.
        pub fn config(&self, key: &str) -> String {
            Self::run(
                self,
                &["-MConfig", "-e", &format!("print $Config{{{key}}}")],
            )
        }

        pub fn embed(&self, opt: EmbedOptions) -> String {
            Self::run(self, &["-MExtUtils::Embed", "-e", &*opt])
        }
    }

    /// Splits ExtUtils::Embed output. Win32 perl quotes paths (`-I"C:\..."`); quotes group
    /// and are removed, backslashes are path separators and kept.
    pub fn split_args(s: &str) -> Vec<String> {
        let (mut args, mut cur, mut quoted, mut has) = (Vec::new(), String::new(), false, false);
        for c in s.chars() {
            match c {
                '"' => {
                    quoted = !quoted;
                    has = true;
                }
                c if c.is_whitespace() && !quoted => {
                    if has {
                        args.push(std::mem::take(&mut cur));
                        has = false;
                    }
                }
                c => {
                    cur.push(c);
                    has = true;
                }
            }
        }
        if has {
            args.push(cur);
        }
        args
    }

    /// ".../libperl540.a" -> (".../", "perl540"); ".../perl540.lib" -> (".../", "perl540")
    pub fn lib_file(arg: &str) -> Option<(String, String)> {
        let path = std::path::Path::new(arg);
        let stem = path.file_stem()?.to_str()?;
        let name = match path.extension()?.to_str()? {
            "a" => stem.strip_prefix("lib").unwrap_or(stem),
            "lib" => stem,
            _ => return None,
        };
        Some((path.parent()?.display().to_string(), name.to_string()))
    }

    /// C integer type from Config.pm -> Rust FFI type.
    pub fn c_type(t: &str) -> &'static str {
        match t {
            "unsigned char" => "::std::ffi::c_uchar",
            "unsigned short" => "::std::ffi::c_ushort",
            "unsigned int" => "::std::ffi::c_uint",
            "unsigned long" => "::std::ffi::c_ulong",
            other => panic!("unsupported perl integer type: {other}"),
        }
    }
}
