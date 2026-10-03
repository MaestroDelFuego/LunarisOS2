pub fn log(message: &str) {
    // The first milestone intentionally keeps the serial layer abstract so the
    // boot environment can be extended without a hard dependency on a specific
    // hardware serial implementation.
    let _ = message;
}
