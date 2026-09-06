fn g(p: &mut i32) {
    *p = *p + 1;
}
fn f() {
    let mut x = 5;
    let r = &x;
    while *r > 3 {
        g(&mut x);
    }
}
