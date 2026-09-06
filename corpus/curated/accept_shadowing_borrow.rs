fn f() {
    let x = 5;
    let r = &x;
    let z = *r;
    let x = 6;
    let w = x + z;
}
