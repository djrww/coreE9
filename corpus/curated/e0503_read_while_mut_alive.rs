fn f() {
    let mut x = 5;
    let m = &mut x;
    let a = x + 1;
    let w = *m + a;
}
