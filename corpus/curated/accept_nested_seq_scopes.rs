fn f() {
    let mut x = 5;
    {
        let a = &x;
        let u = *a;
    }
    {
        let b = &x;
        let v = *b;
    }
    x = 7;
}
