fn h(q: &i32) -> i32 {
    return *q;
}
fn f() {
    let x = 5;
    let a = &x;
    let b = &x;
    let s = *a + *b;
}
