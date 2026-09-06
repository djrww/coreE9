struct S {
    a: i32,
    b: i32,
}
fn f(s: &mut S) {
    let p = &mut s.a;
    let q = &mut s.b;
    *p = 1;
    *q = 2;
}
