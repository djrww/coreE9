fn f() {
    let mut x = 5;
    let m = &mut x;
    {
        let m2 = &mut x;
        *m2 = 2;
    }
    *m = 1;
}
