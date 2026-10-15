<center>
  <img src="./nacre_rs.svg" align="center" alt="alt text">
</center>

# Nacre-rs

(Hopefully) safe wrappers around Perls C-API.

> :warning: WINDOWS PROBLEMS :warning:
> 
> If you build on windows, then there are some problems using `libperl-sys`.
> The official version of the create only supports linux environments.
> By using these patched versions they should also work on windows.
> 
> ```toml
> # Patches for windows suppport.
> [patch.crates-io]
> libperl-macrogen = { git = "https://github.com/philipp-haker/libperl-macrogen", branch = "main"   }
> libperl-sys      = { git = "https://github.com/philipp-haker/libperl-rs",       branch = "master" }
> libperl-config   = { git = "https://github.com/philipp-haker/libperl-rs",       branch = "master" }
> ```
> If you don't want to apply random patches, then you have to use `perl_custom` as the backend. 
>

## Examples

```rust
use nacre_rs as perl;

// initialize a perl system
let sys = perl::Sys::init3(/**/, /**/, /**/);

// Once you have a system you can crete a perl interpreter
let my_perl = sys.new_perl();

```

`perl_sys_term` is called automatically once the system and the interpreters are dropped.
You can safely drop the `Sys` first.
The interpreters keep it alive:

```rust
use nacre_rs as perl;

let sys = perl::Sys::init3(/**/, /**/, /**/);

let my_perl = sys.new_perl().unwrap();

drop(sys);

// my_perl still fully functional

// this will deconstruct and free the interpreter
drop(my_perl);
// Since no sys and no interpreter are active `perl_sys_term` is called here too.

```

While a sys or an interpreter is stil alive,
you can get a new sys to create new interpreters by calling `Sys::get_initialized().unwrap()`.
You can also call `my_perl.sys()` to get the system.

Calling `Sys::init3` while a system is active, will NOT call `perl_sys_init3`, instead it will return the existing one.

## Backends

Two backends are available:
- `perl_custom`:
  Makes the create link itself against perls headers and compile a small c-shim to avoid dealing with c-macros.
  The resulting binary will be smaller.
- `perl_sys`:  
  This backend uses `libperl_sys`.
  It will result in a larger binary and longer compile times.
  The dependency used is a patched version of `libperl_rs`, that contains windows specific fixes.