//! 極小 JSON 值(零依賴)—— 只為 P2 #10「最小 LSP」的 JSON-RPC 訊息框所需。
//! 覆蓋 LSP 訊息實際用到的子集:object / array / string / number / bool / null。
//! 刻意不做浮點運算精確語義 —— LSP 的 number 只作 `id`(整數),這裡用抽象數值。

/// 一個 JSON 值(object 保序,因 LSP 訊息鍵序無關但除錯可讀)。
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    /// `null`。
    Null,
    /// `true` / `false`。
    Bool(bool),
    /// 數值(以 f64 表示;id 只會是整數)。
    Number(f64),
    /// 字串。
    String(String),
    /// 陣列。
    Array(Vec<Value>),
    /// 物件(保序鍵值對)。
    Object(Vec<(String, Value)>),
}

impl Value {
    /// 便捷建構一字串值。
    pub fn string(s: impl Into<String>) -> Value {
        Value::String(s.into())
    }
    /// 便捷建構一數值(通常為整數)。
    pub fn number(n: f64) -> Value {
        Value::Number(n)
    }
    /// 便捷:取得 object 中某鍵的引用。
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Object(kv) => kv.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }
    /// 若為字串,取 &str。
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }
    /// 若為數值,取 f64。
    pub fn as_number(&self) -> Option<f64> {
        match self {
            Value::Number(n) => Some(*n),
            _ => None,
        }
    }
}

/// 字面量序列化(把值寫進緩衝區)。
fn write_string(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

fn write(out: &mut String, v: &Value) {
    match v {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => {
            if n.fract() == 0.0 {
                out.push_str(&format!("{}", *n as i64));
            } else {
                out.push_str(&format!("{}", n));
            }
        }
        Value::String(s) => write_string(out, s),
        Value::Array(a) => {
            out.push('[');
            for (i, x) in a.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write(out, x);
            }
            out.push(']');
        }
        Value::Object(kv) => {
            out.push('{');
            for (i, (k, x)) in kv.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_string(out, k);
                out.push(':');
                write(out, x);
            }
            out.push('}');
        }
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = String::new();
        write(&mut s, self);
        write!(f, "{s}")
    }
}

// ---------------------------------------------------------------------------
// 解析器(遞歸下降)
// ---------------------------------------------------------------------------

struct P<'a> {
    b: &'a [u8],
    pos: usize,
}

impl<'a> P<'a> {
    fn peek(&self) -> Option<u8> {
        self.b.get(self.pos).copied()
    }
    fn bump(&mut self) -> Option<u8> {
        let c = self.b.get(self.pos).copied();
        if c.is_some() {
            self.pos += 1;
        }
        c
    }
    fn skip_ws(&mut self) {
        while let Some(c) = self.peek() {
            if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' {
                self.pos += 1;
            } else {
                break;
            }
        }
    }
    fn expect(&mut self, c: u8) -> Result<(), String> {
        if self.bump() == Some(c) {
            Ok(())
        } else {
            Err(format!("expected '{}' at {}", c as char, self.pos))
        }
    }
    fn parse(&mut self) -> Result<Value, String> {
        self.skip_ws();
        match self.peek() {
            Some(b'{') => self.parse_object(),
            Some(b'[') => self.parse_array(),
            Some(b'"') => Ok(Value::String(self.parse_string()?)),
            Some(b't') => self.parse_lit("true", Value::Bool(true)),
            Some(b'f') => self.parse_lit("false", Value::Bool(false)),
            Some(b'n') => self.parse_lit("null", Value::Null),
            Some(c) if c == b'-' || c.is_ascii_digit() => self.parse_number(),
            _ => Err("unexpected byte".into()),
        }
    }
    fn parse_lit(&mut self, lit: &str, v: Value) -> Result<Value, String> {
        for c in lit.bytes() {
            if self.bump() != Some(c) {
                return Err(format!("expected literal {lit}"));
            }
        }
        Ok(v)
    }
    fn parse_string(&mut self) -> Result<String, String> {
        self.expect(b'"')?;
        let mut out = String::new();
        loop {
            match self.bump() {
                Some(b'"') => break,
                Some(b'\\') => match self.bump() {
                    Some(b'"') => out.push('"'),
                    Some(b'\\') => out.push('\\'),
                    Some(b'/') => out.push('/'),
                    Some(b'n') => out.push('\n'),
                    Some(b'r') => out.push('\r'),
                    Some(b't') => out.push('\t'),
                    Some(b'b') => out.push('\u{8}'),
                    Some(b'f') => out.push('\u{c}'),
                    Some(b'u') => {
                        let hex = self.take(4)?;
                        let cp = u32::from_str_radix(&hex, 16).map_err(|_| "bad \\u")?;
                        out.push(char::from_u32(cp).unwrap_or('\u{FFFD}'));
                    }
                    _ => return Err("bad escape".into()),
                },
                Some(c) => {
                    // 極簡:僅處理 ASCII 參數。
                    out.push(c as char);
                }
                None => return Err("unterminated string".into()),
            }
        }
        Ok(out)
    }
    fn take(&mut self, n: usize) -> Result<String, String> {
        let s = std::str::from_utf8(&self.b[self.pos..self.pos + n]).map_err(|_| "bad utf8")?;
        self.pos += n;
        Ok(s.to_string())
    }
    fn parse_number(&mut self) -> Result<Value, String> {
        let start = self.pos;
        if self.peek() == Some(b'-') {
            self.bump();
        }
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                self.pos += 1;
            } else {
                break;
            }
        }
        if self.peek() == Some(b'.') {
            self.bump();
            while let Some(c) = self.peek() {
                if c.is_ascii_digit() {
                    self.pos += 1;
                } else {
                    break;
                }
            }
        }
        if self.peek() == Some(b'e') || self.peek() == Some(b'E') {
            self.bump();
            if self.peek() == Some(b'+') || self.peek() == Some(b'-') {
                self.bump();
            }
            while let Some(c) = self.peek() {
                if c.is_ascii_digit() {
                    self.pos += 1;
                } else {
                    break;
                }
            }
        }
        let s = std::str::from_utf8(&self.b[start..self.pos]).map_err(|_| "bad number")?;
        let n: f64 = s.parse().map_err(|_| format!("bad number {s}"))?;
        Ok(Value::Number(n))
    }
    fn parse_array(&mut self) -> Result<Value, String> {
        self.expect(b'[')?;
        let mut a = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b']') {
            self.bump();
            return Ok(Value::Array(a));
        }
        loop {
            a.push(self.parse()?);
            self.skip_ws();
            match self.bump() {
                Some(b',') => continue,
                Some(b']') => break,
                _ => return Err("array: expected , or ]".into()),
            }
        }
        Ok(Value::Array(a))
    }
    fn parse_object(&mut self) -> Result<Value, String> {
        self.expect(b'{')?;
        let mut kv = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b'}') {
            self.bump();
            return Ok(Value::Object(kv));
        }
        loop {
            self.skip_ws();
            if self.peek() != Some(b'"') {
                return Err("object: expected key".into());
            }
            let key = self.parse_string()?;
            self.skip_ws();
            self.expect(b':')?;
            let val = self.parse()?;
            kv.push((key, val));
            self.skip_ws();
            match self.bump() {
                Some(b',') => continue,
                Some(b'}') => break,
                _ => return Err("object: expected , or }".into()),
            }
        }
        Ok(Value::Object(kv))
    }
}

/// 解析一個 JSON 值的字節串(允許尾部空白;拒絕殘餘非空白)。
pub fn parse_bytes(bytes: &[u8]) -> Result<Value, String> {
    let mut p = P { b: bytes, pos: 0 };
    let v = p.parse()?;
    p.skip_ws();
    if p.pos != bytes.len() {
        return Err(format!("trailing bytes at {}", p.pos));
    }
    Ok(v)
}

/// 解析一個 UTF-8 JSON 字串。
pub fn parse(s: &str) -> Result<Value, String> {
    parse_bytes(s.as_bytes())
}

/// 便捷:構造 `{ "jsonrpc": "2.0", ... }` 響應/通知的 object(供 lsp 用)。
pub fn object(pairs: Vec<(&str, Value)>) -> Value {
    Value::Object(pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
}
