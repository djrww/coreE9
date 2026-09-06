fn g2(a: i32) -> i32 {
    return a + 1;
}
fn f() {
    let x = 5;
    let r = &x;
    let y = g2(x);
    let z = *r;
}
