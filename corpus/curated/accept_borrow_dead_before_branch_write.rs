fn f() {
    let mut x = 5;
    let r = &x;
    let t = *r;
    if t > 3 {
        x = 6;
    }
}
