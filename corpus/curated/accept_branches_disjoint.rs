fn f() {
    let mut x = 5;
    let r = &x;
    if x > 3 {
        let u = *r;
    } else {
        x = 6;
    }
}
