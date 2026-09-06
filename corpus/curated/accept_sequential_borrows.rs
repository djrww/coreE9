fn f() {
    let mut x = 5;
    let a = &x;
    let u = *a;
    x = 6;
    let b = &x;
    let v = *b;
    let s = u + v + x;
}
