fn g(p: &mut i32) {
    *p = *p + 1;
}
fn f() {
    let mut x = 5;
    let m = &mut x;
    g(&mut x);
    let z = *m;
}
