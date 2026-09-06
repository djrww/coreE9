fn f() {
    let mut x = 5;
    let m = &mut x;
    let m2 = m;
    *m = 1;
}
