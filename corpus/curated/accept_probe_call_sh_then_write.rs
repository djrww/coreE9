fn h(q: &i32) -> i32 {
    return *q;
}
fn f() {
    let mut x = 5;
    let z = h(&x);
    x = 6;
    let w = z + x;
}
