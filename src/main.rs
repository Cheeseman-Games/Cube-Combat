#[cfg(not(target_arch = "wasm32"))]
fn main() {
    let args: Vec<String> = std::env::args().collect();
    cube_combat::platform::run_native(&args);
}

// The wasm build produces a cdylib; an empty binary keeps the crate building
// cleanly when the whole target is compiled.
#[cfg(target_arch = "wasm32")]
fn main() {}