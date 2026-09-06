fn f() {
    let mut x = 5;
    let m = &mut x;
    *m = 1;
    let m2 = &mut x;
    *m2 = 2;
}
