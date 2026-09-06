fn f() {
    let mut x = 5;
    let r = &x;
    let z = *r;
    x = 6;
    let w = x + z;
}
