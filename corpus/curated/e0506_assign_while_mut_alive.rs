fn f() {
    let mut x = 5;
    let m = &mut x;
    x = 6;
    let w = *m;
}
