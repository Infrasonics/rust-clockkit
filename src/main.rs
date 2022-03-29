use cxx;

#[cxx::bridge]
mod ffi {

    struct ConfigReader {
        server: String,
        port: u16,
        timeout: u32,
        phasePanic: u32,
        updatePanic: u32,
    }

    unsafe extern "C++" {
        include!("clockkit/include/ClockKit/bridge.h");

        fn buildPLC(config: ConfigReader) -> SharedPtr<PhaseLockedClock>;
        fn setPhasePanic(clock: SharedPtr<PhaseLockedClock>, dur: i64);
        fn setUpdatePanic(clock: SharedPtr<PhaseLockedClock>, dur: i64);
        fn getValue(clock: SharedPtr<PhaseLockedClock>) -> i64;
    }
    #[namespace = "dex"]
    unsafe extern "C++" {
        include!("clockkit/include/ClockKit/PhaseLockedClock.h");
        type PhaseLockedClock;

        fn isSynchronized(&self) -> bool;
        fn run1(clock: SharedPtr<PhaseLockedClock>);
        fn cancel(clock: SharedPtr<PhaseLockedClock>);
    }

}

impl Default for ffi::ConfigReader {
    fn default() -> Self {
        Self {
            server: "127.0.0.1".to_owned(),
            port: 4444,
            timeout: 1000,
            phasePanic: 5000,
            updatePanic: 5000000,
        }
    }
}

impl ffi::ConfigReader {
    fn read_from(path: impl AsRef<str>) -> Self {
        let config = std::fs::File::open(path.as_ref());
        //TODO read lines and parse
        Self::default()
    }
}

unsafe impl Send for ffi::PhaseLockedClock{}
unsafe impl Sync for ffi::PhaseLockedClock{}


fn main() {
    let config = ffi::ConfigReader::default();
    let plc = ffi::buildPLC(config);
    let plc2 = plc.clone();
    let handle = std::thread::spawn(move || {ffi::run1(plc2); });

    std::thread::sleep_ms(1000);

    for _i in 0..5 {
        if (&plc).isSynchronized() {
            println!("Synced!");
            println!("Value: {}", ffi::getValue(plc.clone()));
        } else {
            println!("Not synced...");
        }
        std::thread::sleep_ms(500);
    }

    ffi::cancel(plc);
    handle.join().unwrap();
}
