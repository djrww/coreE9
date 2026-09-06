fn f() {
    let mut x = 5;
    while x > 3 {
        x = x - 1;
    }
    let r = &x;
    let z = *r;
}
