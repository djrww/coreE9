fn f() {
    let mut x = 5;
    let m = &mut x;
    let r = &m;
    let m2 = m;
    let z = *r;
}
