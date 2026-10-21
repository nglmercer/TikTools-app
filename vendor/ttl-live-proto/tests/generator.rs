// Compile the actual registry generator and execute its synthetic descriptor
// tests, including types not currently present in the pinned schema.
#[allow(dead_code)]
#[path = "../build.rs"]
mod generator;
