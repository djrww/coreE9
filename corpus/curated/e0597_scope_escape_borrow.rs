fn f() {
    let r;
    {
        let x = 5;
        r = &x;
    }
    let z = *r;
}
