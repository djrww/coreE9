fn f() {
    let mut x = 5;
    let m = &mut x;
    let r = &x;
    let z = *r;
    let w = *m;
}
