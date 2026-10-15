/// Checks if the target
#[allow(unused)]
fn is_windows_target() -> bool {
    std::env::var("CARGO_CFG_TARGET_OS").map_or(cfg!(windows), |os| os == "windows")
}

pub fn main() {
    /*
     * Rerun if the buildscript or the shim changed.
     */
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/perl/backend/perl_custom_shim.c");

    /*
     * Replace with call to `perl -MExtUtils::Embed -e ccopts` and parse info from there.
     */
    #[cfg(feature = "perl_custom")]
    {
        cc::Build::new()
            .file("src/backend/perl_custom_shim.c")
            .include(r"C:\emp2exe-env\emp2exe-win-amd64-5.40\perl\lib\CORE")
            .flag("-std=c99")
            .flag("-mms-bitfields")
            .define("WIN32", None)
            .define("WIN64", None)
            .define("PERL_TEXTMODE_SCRIPTS", None)
            .define("MULTIPLICITY", None)
            .define("PERL_IMPLICIT_SYS", None)
            .define("USE_PERLIO", None)
            .define("__USE_MINGW_ANSI_STDIO", None)
            .compile("nacre_shim");

        // Perl libperl: import lib (perl540.lib, generated from perl540.dll
        // via dumpbin/lib /def:) lives in the perl distribution's CORE dir.
        if is_windows_target() {
            println!(
                "cargo:rustc-link-search=native=C:\\emp2exe-env\\emp2exe-win-amd64-5.40\\perl\\lib\\CORE"
            );
            println!("cargo:rustc-link-lib=dylib=perl540");
        } else {
            println!(
                "cargo:rustc-link-search=native=/emp2exe-env/emp2exe-win-amd64-5.40/perl/lib/CORE"
            );
            println!("cargo:rustc-link-lib=dylib=perl540");
        }
    };
}
