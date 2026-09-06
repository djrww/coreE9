fn k(p: &mut i32) {
    *p = *p + 1;
}
fn f() {
    let mut x = 5;
    k(&mut x);
    k(&mut x);
    let z = x + 1;
}
