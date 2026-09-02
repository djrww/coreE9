# -*- coding: utf-8 -*-
# 在 src/r0.rs 的 mod tests 中追加 r0_parse 測試
TESTS = r'''
    #[test]
    fn r0_parse_legal_roundtrip() {
        // 附錄 B 的全語法面:item / 語句 / 全部運算符優先級 / 借用 / 字段 / 索引 / 調用。
        let src = r#"fn main() {
  let mut x = 1;
  let r = &mut x;
  let y = x + 2 * 3 - 4 / 2 % 3;
  let b = x < 1 || y >= 2 && x != 3;
  x = y;
  if x == 1 { f(x, y); } else { g(); }
  while x < 10 { x = x + 1; }
  loop { x = x + 1; }
  return x;
}
struct P { a: int, b: &mut int, c: [int], }
fn q(v: int, w: &int) -> int { let k: int = v; return k; }"#;
        let t = r0_parse(src).expect("legal R0 must parse");
        assert!(!t.has_error(), "legal program must have no ERROR node");
        assert!(
            t.unsupported_spans().is_empty(),
            "legal program must have no Unsupported: {:?}",
            t.unsupported_spans()
        );
        assert_eq!(t.unparse(), src, "byte-exact roundtrip");
        assert!(t.validate_continuity().is_ok());
        assert!(t.validate_tree_shapes().is_ok());
        assert!(t.laminar_ok());
        // 結構面:具名節點存在(函數/塊/語句/表達式)
        assert!(
            t.nodes.iter().any(|n| n.kind == R0Kind::FnItem),
            "fn items present"
        );
        assert!(
            t.nodes.iter().any(|n| n.kind == R0Kind::StructItem),
            "struct item present"
        );
        assert!(
            t.nodes.iter().any(|n| n.kind == R0Kind::IfStmt),
            "if stmt present"
        );
        assert!(
            t.nodes.iter().any(|n| n.kind == R0Kind::UnaryExpr),
            "unary expr present"
        );
    }

    #[test]
    fn r0_parse_garbage_totalization() {
        // 總化:任何輸入(含非法字節 / 半截構造 / 亂語法)都產出樹,且樹可驗證性質成立。
        let samples = [
            "",
            "@@|",
            "fn",
            "fn f(",
            "fn f() { let x = ; }",
            "fn f() { if x { } else }",
            "struct S { a: }",
            "fn f() { while { } }",
            "let x = (1;",
            "fn f() { & & & x; }",
            "}}}",
            "fn f() { let x = { { { 1; } }",
            "\x01\x02\xff\xfe",
            "fn f() { let s = r#\"unterminated;",
        ];
        for src in samples {
            let t = r0_parse(src)
                .unwrap_or_else(|e| panic!("totalization violated for {:?}: {:?}", src, e));
            assert!(t.total_nodes() >= 1);
            assert!(
                t.validate_continuity().is_ok(),
                "continuity must hold on {:?}",
                src
            );
            assert!(t.validate_tree_shapes().is_ok());
            assert!(t.laminar_ok());
            assert_eq!(t.unparse(), src, "roundtrip must hold on {:?}", src);
        }
    }

    #[test]
    fn r0_parse_unsupported_nodes() {
        // 節點級 unsupported(§9 如實申報:note + 精確 span,而非假裝覆蓋)。
        let cases: &[(&str, &str)] = &[
            ("fn f() { let c = |a| a; }", "閉包"),
            ("fn f() { match x { _ => {} } }", "match"),
            ("trait T { fn f(); }", "trait"),
            ("fn f() { println!(x); }", "宏調用"),
            ("fn f(x: &'a int) {}", "生命週期"),
            ("fn f() { let v: Vec<int> = v; }", "泛型實參"),
            ("fn f() { let a = <flag>; }", "非法符號"),
        ];
        for (src, needle) in cases {
            let t = r0_parse(src).expect("unsupported constructs still parse (totalization)");
            let notes = t.unsupported_spans();
            assert!(
                notes.iter().any(|(_, n)| n.contains(needle)),
                "case {:?}: expected note containing {:?}, got {:?}",
                src,
                needle,
                notes
            );
            assert!(t.validate_continuity().is_ok());
            assert_eq!(t.unparse(), src);
        }
    }

    #[test]
    fn r0_parse_depth_honest() {
        // 深嵌套越界:如實申報 Err(Depth),永不 panic。
        let deep = format!("fn main() {{ let x = {}1; }}", "{ ".repeat(300));
        let r = r0_parse(&deep);
        match r {
            Err(R0ParseIssue::Depth) => {}
            Ok(t) => assert!(t.has_error() || t.total_nodes() > 0),
            Err(R0ParseIssue::Syntax) => panic!("Depth must be reported, not Syntax"),
        }
        // 正常深程式沒問題
        let okdeep = format!("fn main() {{ {} }}", "{ x; }".repeat(60));
        let t = r0_parse(&okdeep).expect("normal depth must parse");
        assert!(t.total_nodes() > 100);
        assert!(!t.has_error());
    }

    #[test]
    fn r0_parse_determinism_named_sexp() {
        let src = "fn f() { let x = 1; return x; }";
        let a = r0_parse(src).unwrap();
        let b = r0_parse(src).unwrap();
        assert_eq!(a.named_sexp(), b.named_sexp(), "deterministic named projection");
        assert!(
            a.named_sexp().starts_with("(root(fn_item"),
            "named sexp must be structural: {}",
            a.named_sexp()
        );
    }
'''
path = 'src/r0.rs'
s = open(path, encoding='utf-8').read()
marker = "#[test]\n    fn r0_lalr1_clean_checks()"
assert s.count(marker) == 1
# 在該測試的結尾「}」後插入:找到該測試函數結尾(下一個 "}" 後跟 "\n}")
idx = s.index(marker)
# 找到測試體結束:從 marker 向後找 "}\n}\n" (內層閉合 + 函數閉合)
tail = s.index('    }\n}\n', idx) + len('    }\n}\n')
s = s[:tail] + TESTS + s[tail:]
open(path, 'w', encoding='utf-8').write(s)
print('ok tests appended')
