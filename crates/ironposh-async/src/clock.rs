// A small monotonic clock abstraction.
//
// `std::time::Instant` is not available on `wasm32-unknown-unknown` and will
// panic at runtime ("time not implemented on this platform"). Use `web_time`
// on wasm instead.

#[cfg(target_arch = "wasm32")]
pub type Instant = web_time::Instant;

#[cfg(not(target_arch = "wasm32"))]
pub type Instant = std::time::Instant;
