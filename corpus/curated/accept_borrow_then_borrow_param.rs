fn h(q: &i32) -> i32 {
    return *q;
}
fn f() {
    let v = 5;
    let z = h(&v);
    let w = v + z;
}
