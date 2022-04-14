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

For further details see the [clockkit repository](https://github.com/camilleg/clockkit).

# About this crate
The provided API is for the client side only. For the corresponding server see
the [clockkit repository](https://github.com/camilleg/clockkit).

# Bundled Version Info
The clockkit C++ files included in this crate are from commit
a7856021da846988d022879c95ec745caa5ae9e8.
