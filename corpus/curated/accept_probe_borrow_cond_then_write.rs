fn f() {
    let mut x = 5;
    let r = &x;
    if *r > 3 {
        x = 6;
    }
}
