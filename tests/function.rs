use clockkit as ck;
use std::{thread, time};

#[cfg(not(feature = "t_server_manual"))]
use std::process::Command;

/* Uses the bold assumption that after a few tries the clock must be synchronized when server and
 * client are running on localhost. This is a very simple integration check whether the FFI is
 * working properly */
#[test]
fn client_synchronizes() {
    #[cfg(not(feature = "t_server_manual"))]
    let mut server: std::process::Child;

    #[cfg(not(feature = "t_server_manual"))]
    {
        if !cfg!(target_os = "linux") {
            panic!("Bundled server only available for Linux");
        }
        // start the bundled server
        if cfg!(target_arch = "x86_64") {
            server = Command::new("tests/ckserver-amd64-linux")
                .arg("4444")
                .spawn()
                .expect("Failed to start clockkit server");
        } else {
            panic!("Bundled server not available for target_arch, try starting it manually and use feature `t_server_manual`");
        }
        thread::sleep(time::Duration::from_millis(100));
    }

    let mut clock = ck::PhaseLockedClock::default();
    clock.start();
    thread::sleep(time::Duration::from_millis(234));
    clock.is_synchronized();
    clock
        .get_value()
        .expect("Failed to get value from clockkit server");
    assert!(clock.is_synchronized());

    clock.join().unwrap();

    #[cfg(not(feature = "t_server_manual"))]
    {
        server.kill().expect("Error terminating clockkit server");
    }
}
