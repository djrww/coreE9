use cl0r0::parse::parse;
fn main() {
    let cut = "fn g() {\n  while &mut g(&z == k()) < 21 {\n  if trulet \n  let w;\n}\n}\n";
    let t = parse(cut).unwrap();
    for (i, n) in t.nodes.iter().enumerate() {
        println!(
            "{:3} {:12} {:?} children={:?}",
            i,
            format!("{:?}", n.kind),
            n.span,
            n.children
        );
    }
}
