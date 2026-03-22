#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Only try valid UTF-8
    if let Ok(source) = std::str::from_utf8(data) {
        // The compiler must never panic on any input.
        // It may return Ok or Err, but never panic.
        let _ = basalt_core::compile(source);
    }
});
