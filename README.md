# About clockkit
Clockkit provides timestamps to distributed networked PCs with guaranteed
bounds on latency and jitter, typically under 10 microseconds, as described in
the conference paper Synchronous data collection from diverse hardware.

It runs on Linux, Windows, and Raspi, and needs neither extra hardware nor
elevated privileges.

It can measure a system's realtime behavior, by providing a common time
reference for events recorded by different sensors (audio, video, gamepad, GPS,
SMS, MIDI, biometrics), and for triggering outputs (audio, video, LEDs, servos,
motion bases). It did this originally for a full-motion driving simulator with
eye tracking and a quickly churning set of other sensors and outputs, for over
a decade.

For further details see the [clockkit
repository](https://github.com/camilleg/clockkit).

# About this crate
Currently this crate does package a server (amd64) for testing purposes, but the provided API
is for the client side only.

## Nightly only
At the moment this crate is nightly only due to the need for `atomic_mut_ptr` to
interact with the wrapped code.

## Features of this crate
- `t_server_manual`: Do not attempt to start the included server for the test,
  but expect it to be started manually before tests run.
