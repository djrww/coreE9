fn f() {
    let mut x = 5;
    let r = &x;
    x = 6;
    let z = *r;
}
