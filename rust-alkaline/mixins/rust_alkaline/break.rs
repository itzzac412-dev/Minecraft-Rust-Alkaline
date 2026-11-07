// Runs before the native method. Returning unit leaves its body/result intact.
fn before_break(pos: [i32; 3]) {
    crate::game::rust_alkaline::before_break(pos);
}
