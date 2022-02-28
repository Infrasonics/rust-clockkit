//! Accurate distributed timestamps.
//!
//! Bindings to [clockkit](https://github.com/camilleg/clockkit).
//! Currently requires `nightly` for the use of `feature(atomic_mut_ptr)`.
//!
//! Clockkit provides timestamps to distributed networked PCs
//! with guaranteed bounds on latency and jitter, typically under 10 microseconds,
//! as described in the conference paper
//! [Synchronous data collection from diverse hardware](https://github.com/camilleg/clockkit/blob/main/dsceu04.pdf).
//!
//! It runs on Linux, Windows, and Raspi, and needs neither extra hardware nor elevated privileges.
//!
//! It can measure a system's realtime behavior, by providing a common time reference for events recorded by different sensors
//! (audio, video, gamepad, GPS, SMS, MIDI, biometrics), and for triggering outputs (audio, video, LEDs, servos, motion bases).
//!
//! Originally created for a full-motion
//! [driving simulator](https://web.archive.org/web/20170517201424/http://www.isl.uiuc.edu/Labs/Driving%20Simulator/Driving%20Simulator.html)
//! with eye tracking and a quickly churning set of other sensors and outputs, for over a decade.

#![feature(atomic_mut_ptr)]

use chrono::{Duration, NaiveDateTime};
use std::convert::TryInto;
use std::default::Default;
use std::ffi::{c_void, CString};
use std::io;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::sync::atomic::{AtomicPtr, AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use thiserror::Error;

use clockkit_sys as cks;

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
}

fn make_timestamp(val: i64) -> Result<NaiveDateTime, Error> {
    let sec: i64 = val / 1_000_000;
    let usec: Result<u32, _> = match (val % 1_000_000).try_into() {
        Ok(x) => Ok(x),
        Err(_) => Err(Error::Overflow),
    };
    if let Ok(us) = usec {
        Ok(NaiveDateTime::from_timestamp(sec, us * 1000)) // second value are nanoseconds
    } else {
        Err(Error::Overflow)
    }
}

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
/// # use clockkit as ck;
/// # use std::{thread,time};
/// let mut clock = ck::PhaseLockedClock::default();
/// clock.start();
/// thread::sleep(time::Duration::from_millis(234));
/// clock.get_value().expect("Failed to get value from clockkit server");
/// clock.join().unwrap();
/// ```
pub struct PhaseLockedClock {
    /// Handle to the underlying C++ implementation.
    clock: Arc<AtomicPtr<cks::PhaseLockedClock>>,
    /// Atomic to end the infinite loop within the clock.
    end: Arc<AtomicU8>,
    /// Handle to the thread the PLC runs in.
    handle: Mutex<Option<thread::JoinHandle<()>>>,
}

impl Default for PhaseLockedClock {
    /// Creates a PLC with default values.
    ///
    /// This represents the default in a valid configuration file format:
    /// ```conf
    /// server:localhost
    /// port:4444
    /// timeout:1000
    /// phasePanic:5000
    /// updatePanic:5000000
    /// ```
    fn default() -> Self {
        let rawlocalhost: Vec<u8> = "localhost".as_bytes().into();

        let localhost = unsafe { CString::from_vec_unchecked(rawlocalhost) };

        let mut reader = cks::ConfigReader {
            server: localhost.as_ptr(),
            port: cks::ConfigReader_defaultPort,
            timeout: cks::ConfigReader_defaultTimeout,
            phasePanic: cks::ConfigReader_defaultPhasePanic,
            updatePanic: cks::ConfigReader_defaultUpdatePanic,
        };

        let clock_ptr = unsafe { reader.buildPLC() };
        PhaseLockedClock {
            clock: Arc::new(AtomicPtr::new(clock_ptr)),
            end: Arc::new(AtomicU8::new(false.into())),
            handle: Mutex::new(None),
        }
    }
}

impl PhaseLockedClock {
    /// Creates a PLC with the given values.
    pub fn new(address: impl AsRef<str>, port: u16) -> Self {
        let addr_raw: Vec<u8> = address.as_ref().as_bytes().into();

        let addr = unsafe { CString::from_vec_unchecked(addr_raw) };

        let mut reader = cks::ConfigReader {
            server: addr.as_ptr(),
            port: port.into(),
            timeout: cks::ConfigReader_defaultTimeout,
            phasePanic: cks::ConfigReader_defaultPhasePanic,
            updatePanic: cks::ConfigReader_defaultUpdatePanic,
        };

        let clock_ptr = unsafe { reader.buildPLC() };
        PhaseLockedClock {
            clock: Arc::new(AtomicPtr::new(clock_ptr)),
            end: Arc::new(AtomicU8::new(false.into())),
            handle: Mutex::new(None),
        }
    }

    /// Run the PLC in its own thread
    pub fn start(&self) {
        if let Ok(mut guard) = self.handle.lock() {
            // Only start the clock if there is no handle present, otherwise it's running.
            if (*guard).is_none() {
                let cl = self.clock.clone();
                let end = self.end.clone();
                *guard = Some(thread::spawn(move || unsafe {
                    cks::PhaseLockedClock_run((*cl).load(Ordering::Relaxed), end.as_mut_ptr())
                }))
            }
        } else {
            panic!("Unable to start PhaseLockedClock due to poisened mutex");
        };
    }

    /// Join the clock's thread explicitly.
    ///
    /// This can be done explicitly before the clock is `drop()`ped, otherwise drop tries to join
    /// the thread which will block and might panic.
    ///
    /// A detailed opinion on why waiting for `drop()` to join the thread might not be the best
    /// option can be read here:
    /// <https://stackoverflow.com/questions/41331577/joining-a-thread-in-a-method-that-takes-mut-self-like-drop-results-in-cann/42791007#42791007>
    pub fn join(self) -> std::thread::Result<()> {
        {
            let end: *mut u8 = (*self.end).as_mut_ptr();
            // This sets the break condition of the infinite loop in PhaseLockedClock see ::run() for details
            // The value is set to false in the constructor and only written here once to stop the
            // clock. No other accesses are allowed by the interface in this crate.
            unsafe {
                *end = true.into();
            }
        }
        if let Ok(mut guard) = self.handle.lock() {
            match (*guard).take() {
                Some(h) => h.join(),
                None => Ok(()),
            }
        } else {
            panic!("Clean shutdown of the PhaseLockedClock failed. Mutex poisoned");
        }
    }

    /// Create a new PLC with the settings from `path`.
    ///
    /// The config file format looks like:
    /// ```conf
    /// server:localhost
    /// port:4444
    /// timeout:1000
    /// phasePanic:5000
    /// updatePanic:5000000
    /// ```
    pub fn from_config_file<F>(path: F) -> Result<Self, io::Error>
    where
        F: AsRef<Path>,
    {
        if !path.as_ref().is_file() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "Config file does not exist",
            ));
        }

        // XXX this is platform specific, check out
        // https://stackoverflow.com/questions/43083544/how-can-i-convert-osstr-to-u8-vecu8-on-windows
        let rawpath: Vec<u8> = path.as_ref().as_os_str().as_bytes().into();
        let rawlocalhost: Vec<u8> = "localhost".as_bytes().into();

        let localhost = unsafe { CString::from_vec_unchecked(rawlocalhost) };

        let mut reader = cks::ConfigReader {
            server: localhost.as_ptr(),
            port: cks::ConfigReader_defaultPort,
            timeout: cks::ConfigReader_defaultTimeout,
            phasePanic: cks::ConfigReader_defaultPhasePanic,
            updatePanic: cks::ConfigReader_defaultUpdatePanic,
        };

        let conf_reader_ptr: *mut cks::ConfigReader = &mut reader;

        let config = unsafe { CString::from_vec_unchecked(rawpath) };
        let success: bool = unsafe { cks::ConfigReader_readFrom(conf_reader_ptr, config.as_ptr()) };
        if !success {
            return Err(std::io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Invalid data in clockkit config at '{}'",
                    path.as_ref().display()
                ),
            ));
        }
        let clock_ptr = unsafe { cks::ConfigReader_buildPLC(conf_reader_ptr) };
        Ok(PhaseLockedClock {
            clock: Arc::new(AtomicPtr::new(clock_ptr)),
            end: Arc::new(AtomicU8::new(false.into())),
            handle: Mutex::new(None),
        })
    }

    /// Check whether the PLC is synchronized.
    pub fn is_synchronized(&self) -> bool {
        unsafe { (*(*self.clock).load(Ordering::Acquire)).inSync_ }
    }

    /// Query the offset to the reference clock.
    pub fn get_offset(&self) -> Result<Duration, Error> {
        if self.is_synchronized() {
            Ok(Duration::microseconds(unsafe {
                cks::PhaseLockedClock_getOffset((*self.clock).load(Ordering::Acquire))
            }))
        } else {
            Err(Error::OutOfSync)
        }
    }

    /// Get the current value of the PLC.
    pub fn get_value(&self) -> Result<NaiveDateTime, Error> {
        if self.is_synchronized() {
            make_timestamp(unsafe {
                let ptr: *mut cks::PhaseLockedClock = (*self.clock).load(Ordering::Acquire);
                cks::PhaseLockedClock_getValue(ptr as *mut c_void)
            })
        } else {
            Err(Error::OutOfSync)
        }
    }

    /// Set the threshold for the phase panic.
    pub fn set_phase_panic(&self, panic: Duration) -> Result<(), Error> {
        unsafe {
            cks::PhaseLockedClock_setPhasePanic(
                (*self.clock).load(Ordering::Acquire),
                panic.num_microseconds().ok_or(Error::Overflow)?,
            );
        }
        Ok(())
    }

    /// Set the threshold for the update panic.
    pub fn set_update_panic(&self, panic: Duration) -> Result<(), Error> {
        unsafe {
            cks::PhaseLockedClock_setUpdatePanic(
                (*self.clock).load(Ordering::Acquire),
                panic.num_microseconds().ok_or(Error::Overflow)?,
            );
        }
        Ok(())
    }
}

impl Drop for PhaseLockedClock {
    fn drop(&mut self) {
        if let Ok(mut guard) = self.handle.lock() {
            if (*guard).is_some() {
                {
                    let end: *mut u8 = (*self.end).as_mut_ptr();
                    // This sets the break condition of the infinite loop in PhaseLockedClock see ::run() for details
                    // The value is set to false in the constructor and only written here once to stop the
                    // clock. No other accesses are allowed by the interface in this crate.
                    unsafe {
                        *end = true.into();
                    }
                }
                match (*guard).take() {
                    Some(h) => h.join(),
                    None => Ok(()),
                }
                .expect("failed to join clock thread");
                // Actually call the c++ dtor
                unsafe {
                    cks::PhaseLockedClock_PhaseLockedClock_destructor(
                        (*self.clock).load(Ordering::Acquire),
                    );
                }
            }
        }
    }
}

/// The sending of a clock will only add a little delay and should not be a problem in general. Too
/// many movements might end up causing the clock to go out of sync or result in inaccurate values.
unsafe impl Send for PhaseLockedClock {}

unsafe impl Sync for PhaseLockedClock {}
