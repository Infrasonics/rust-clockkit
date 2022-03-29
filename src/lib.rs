///! Accurate distributed timestamps.
///!
///! Bindings to [clockkit](https://github.com/camilleg/clockkit).
///! Currently requires `nightly` for the use of `feature(atomic_mut_ptr)`.
///!
///! Clockkit provides timestamps to distributed networked PCs
///! with guaranteed bounds on latency and jitter, typically under 10 microseconds,
///! as described in the conference paper
///! [Synchronous data collection from diverse hardware](https://github.com/camilleg/clockkit/blob/main/dsceu04.pdf).
///!
///! It runs on Linux, Windows, and Raspi, and needs neither extra hardware nor elevated privileges.
///!
///! It can measure a system's realtime behavior, by providing a common time reference for events recorded by different sensors
///! (audio, video, gamepad, GPS, SMS, MIDI, biometrics), and for triggering outputs (audio, video, LEDs, servos, motion bases).
///!
///! Originally created for a full-motion
///! [driving simulator](https://web.archive.org/web/20170517201424/http://www.isl.uiuc.edu/Labs/Driving%20Simulator/Driving%20Simulator.html)
///! with eye tracking and a quickly churning set of other sensors and outputs, for over a decade.

use std::{sync::Mutex, thread::JoinHandle, time::Duration, path::Path};

use chrono::{DateTime, NaiveDateTime, Utc};
use cxx::{self, SharedPtr};
use thiserror::Error;

/// Things that can go wrong.
#[derive(Error, Debug)]
pub enum Error {
    /// The clock became out of sync.
    #[error("Clock out of sync")]
    OutOfSync,
    /// The request with the server timed out.
    #[error("Server request timed out")]
    Timeout,
    /// The internal representation overflowed.
    #[error("Overflow")]
    Overflow,
    /// Could not read config file.
    #[error("Could not read config file")]
    ConfigRead(#[from] std::io::Error),
    /// Invalid configuration value.
    #[error("Invalid configuration value")]
    ConfigValue,
    /// Invalid configuration key.
    #[error("Invalid configuration key")]
    ConfigKey(String),
}

#[cxx::bridge]
mod ffi {

    /// Example:
    /// ```
    /// use clockkit;
    /// let clock = clockkit::Config::default()
    ///     .server("10.10.10.20".to_string())
    ///     .port(1234)
    ///     .build_clock();
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
    /// This represents the default in a valid configuration file format:
    /// ```conf
    /// server:localhost
    /// port:4444
    /// timeout:1000
    /// phasePanic:5000
    /// updatePanic:5000000
    /// ```
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
    /// Create a new PLC config with the settings from `path`.
    ///
    /// The config file format looks like:
    /// ```conf
    /// server:localhost
    /// port:4444
    /// timeout:1000
    /// phasePanic:5000
    /// updatePanic:5000000
    /// ```
    pub fn from_config_file(path: impl AsRef<Path>) -> Result<Self, Error> {
        let mut res = Self::default();

        let config = std::fs::read_to_string(path.as_ref())?;
        for line in config.lines() {
            if line.starts_with('#') {
                continue
            }
            let mut parts = line.trim().splitn(2, ':');
            if let Some(key) = parts.next() {
                if let Some(ref val) = parts.next() {
                    match key {
                        "server" => res.server = val.to_string(),
                        "port" => res.port = val.parse().map_err(|_| Error::ConfigValue)?,
                        "timeout" => res.timeout = val.parse().map_err(|_| Error::ConfigValue)?,
                        "phasePanic" => {
                            res.phasePanic = val.parse().map_err(|_| Error::ConfigValue)?
                        }
                        "updatePanic" => {
                            res.updatePanic = val.parse().map_err(|_| Error::ConfigValue)?
                        }
                        _ => return Err(Error::ConfigKey(key.to_string())),
                    }
                }
            }
        }
        Ok(res)
    }

    pub fn build_clock(self) -> PhaseLockedClock {
        PhaseLockedClock {
            ptr: ffi::buildPLC(self),
            handle: Mutex::new(None),
        }
    }

    pub fn server(mut self, server: String) -> Self {
        self.server = server;
        self
    }

    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    pub fn phase_panic(mut self, phase_panic: u32) -> Self {
        self.phasePanic = phase_panic;
        self
    }

    pub fn update_panic(mut self, update_panic: u32) -> Self {
        self.updatePanic = update_panic;
        self
    }
}

pub type Config = ffi::ConfigReader;

/// A clock locking its phase and frequency to a reference clock.
///
/// This class reads two clocks, a primary clock and a reference clock,
/// both assumed to run at 1000000 Hz.
/// The primary clock is usually a HighResolutionClock, whereas
/// the reference clock is usually a ClockClient.
/// It makes a variable frequency clock, whose phase and frequency
/// it keeps locked to those of the reference clock.
///
/// Example:
/// ```no_run
/// # use clockkit;
/// # use std::{thread,time};
/// let mut clock = clockkit::Config::default().build_clock();
/// clock.start();
/// thread::sleep(time::Duration::from_millis(234));
/// clock.get_value().expect("Failed to get value from clockkit server");
/// ```
pub struct PhaseLockedClock {
    ptr: SharedPtr<ffi::PhaseLockedClock>,
    handle: Mutex<Option<JoinHandle<()>>>,
}

/// Helper function to create a NaiveDateTime from a timestamp in μs.
fn make_timestamp(usec: i64) -> Result<NaiveDateTime, Error> {
    let sec: i64 = usec / 1_000_000;
    let usec: Result<u32, _> = match (usec % 1_000_000).try_into() {
        Ok(x) => Ok(x),
        Err(_) => Err(Error::Overflow),
    };
    usec.and_then(|us| {
        NaiveDateTime::from_timestamp_opt(sec, us * 1000) // second value are nanoseconds
            .ok_or(Error::Overflow)
    })
}

impl PhaseLockedClock {
    pub fn get_value(&self) -> Result<chrono::DateTime<Utc>, Error> {
        make_timestamp(ffi::getValue(self.ptr.clone())).map(|ts| DateTime::<Utc>::from_utc(ts, Utc))
    }

    /// Check whether the PLC is synchronized.
    pub fn is_synchronized(&self) -> bool {
        self.ptr.isSynchronized()
    }

    /// Run the PLC in its own thread
    pub fn start(&self) {
        if let Ok(mut guard) = self.handle.lock() {
            // Only start the clock if there is no handle present, otherwise it's running.
            if (*guard).is_none() {
                let plc = self.ptr.clone();
                *guard = Some(std::thread::spawn(move || ffi::run1(plc)))
            }
        } else {
            panic!("Unable to start PhaseLockedClock due to poisened mutex");
        };
    }

    /// Stop the PLC.
    pub fn stop(&self) {
        let plc = self.ptr.clone();
        ffi::cancel(plc);
    }

    /// Set the threshold for the phase panic.
    ///
    /// phasePanic: A PhaseLockedClock whose offset exceeds this,
    /// relative to its reference clock, declares itself out of sync.
    /// Default: 5ms
    pub fn set_phase_panic(&mut self, dur: Duration) {
        let dur = dur
            .as_micros()
            .try_into()
            .expect("Duration greater than i64");
        let plc = self.ptr.clone();
        ffi::setPhasePanic(plc, dur)
    }

   /// Set the threshold for the update panic.
   ///
   /// updatePanic: A PhaseLockedClock that hasn't updated successfully
   /// for longer than this declares itself out of sync.
   /// Default: 5s
    pub fn set_update_panic(&mut self, dur: Duration) {
        let dur = dur
            .as_micros()
            .try_into()
            .expect("Duration greater than i64");
        let plc = self.ptr.clone();
        ffi::setUpdatePanic(plc, dur)
    }
}

impl Drop for PhaseLockedClock {
    /// `drop` tries to join the thread which will block and might panic.
    ///
    /// A detailed opinion on why waiting for `drop()` to join the thread might not be the best
    /// option can be read here:
    /// <https://stackoverflow.com/questions/41331577/joining-a-thread-in-a-method-that-takes-mut-self-like-drop-results-in-cann/42791007#42791007>
    fn drop(&mut self) {
        if let Ok(mut guard) = self.handle.lock() {
            if (*guard).is_some() {
                self.stop();
                match (*guard).take() {
                    Some(h) => h.join(),
                    None => Ok(()),
                }
                .expect("failed to join clock thread");
            }
        }
    }
}

/// The sending of a clock will only add a little delay and should not be a problem in general. Too
/// many movements might end up causing the clock to go out of sync or result in inaccurate values.
unsafe impl Send for ffi::PhaseLockedClock {}

unsafe impl Sync for ffi::PhaseLockedClock {}

