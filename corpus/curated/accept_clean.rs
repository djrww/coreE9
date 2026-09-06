fn add(a: i32, b: i32) -> i32 {
    return a + b;
}
fn f() {
    let mut i = 0;
    let mut acc = 0;
    while i < 10 {
        acc = acc + add(i, 1);
        i = i + 1;
    }
    if acc > 5 {
        acc = acc - 1;
    } else {
        acc = acc + 1;
    }
}
