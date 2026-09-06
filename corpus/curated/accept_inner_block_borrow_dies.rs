fn f() {
    let mut x = 5;
    {
        let r = &x;
        let z = *r;
    }
    x = 6;
}
