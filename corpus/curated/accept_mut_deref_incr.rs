fn f() {
    let mut x = 5;
    let m = &mut x;
    *m = *m + 1;
}
