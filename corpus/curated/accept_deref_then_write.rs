fn f() {
    let mut x = 5;
    let m = &mut x;
    let a = *m;
    x = 6;
    let w = a + x;
}
