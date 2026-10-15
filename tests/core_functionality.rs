//! This module tests the core functionality, that is:
//! - Initializing the system
//! - Create an interpreter
//! - Parse some code
//! - Run the interpreter
//! - Teardown
//!

use nacre_rs as perl;
use rusty_fork::rusty_fork_test;

rusty_fork_test! {
    /// Creates a system, interpreter and parses a hello world.
    #[test]
    fn init_system() {
        let args = Vec::from_iter(std::env::args_os());
        let (sys, _) = perl::Sys::init3(&args, std::env::vars_os()).expect("No System should be running.");

        let my_perl = sys.new_perl();

        assert!(my_perl.is_some(), "No interpreter was created.");
        let mut my_perl = my_perl.unwrap();

        drop(sys);

        let x = my_perl.parse_default_no_env(&["", "-e", include_str!("test_scripts/hello_world.pl")]);

        assert_eq!(None, x.err(), "Parsing failed.");
    }

    /// Tests that sys is a singleton.
    #[test]
    fn no_double_sys_init() {
        let args = Vec::from_iter(std::env::args_os());
        let envs = Vec::from_iter(std::env::vars_os());

        let tmp = perl::Sys::init3(&args, envs.iter().map(|(a, b)| (a, b)));
        assert!(tmp.is_ok(), "System could not be allocated");
        let (sys1, created) = tmp.unwrap();

        assert!(created, "No system was created.");

        let tmp = perl::Sys::init3(&args, envs.iter().map(|(a, b)| (a, b)));
        assert!(tmp.is_ok(), "A system is active but none was received");
        let (sys2, created) = tmp.unwrap();

        assert!(!created, "A second system was created.");

        assert_eq!(sys1, sys2, "The systems have different drop-guards.");

        // this drops sys1
        assert_eq!(
            Some(sys1),
            perl::Sys::get_initialized(),
            "A system was active but it was not returned as initilaized"
        );

        drop(sys2);

        assert_eq!(
            None,
            perl::Sys::get_initialized(),
            "No system was active but one was returned."
        );

        assert_eq!(Err(perl::SysTerminated), perl::Sys::init3(&args, envs.iter().map(|(a, b)| (a, b))));
    }
}