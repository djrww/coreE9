fn f() {
    let mut x = 5;
    let m = &mut x;
    while *m > 3 {
        x = 6;
    }
}
