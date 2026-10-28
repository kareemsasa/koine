//! Bounded, strict text KeyValues subset. No includes, directives or I/O.
use koine_core::Result;
use std::collections::BTreeMap;
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Kv {
    Text(String),
    Object(BTreeMap<String, Kv>),
}
impl Kv {
    pub fn object(&self) -> Result<&BTreeMap<String, Kv>> {
        match self {
            Self::Object(v) => Ok(v),
            _ => Err("expected object".into()),
        }
    }
    pub fn text(&self) -> Result<&str> {
        match self {
            Self::Text(v) => Ok(v),
            _ => Err("expected text".into()),
        }
    }
    pub fn get(&self, key: &str) -> Result<&Kv> {
        self.object()?
            .get(&key.to_ascii_lowercase())
            .ok_or_else(|| format!("missing field {key}"))
    }
}
pub fn parse(bytes: &[u8]) -> Result<Kv> {
    if bytes.len() > 1_048_576 {
        return Err("oversized-keyvalues".into());
    }
    let source = std::str::from_utf8(bytes).map_err(|_| "unsupported-encoding")?;
    struct Parser<'a> {
        s: &'a [u8],
        pos: usize,
        tokens: usize,
    }
    impl Parser<'_> {
        fn space(&mut self) {
            loop {
                while self.pos < self.s.len() && self.s[self.pos].is_ascii_whitespace() {
                    self.pos += 1
                }
                if self.s.get(self.pos..self.pos + 2) == Some(b"//") {
                    while self.pos < self.s.len() && self.s[self.pos] != b'\n' {
                        self.pos += 1
                    }
                } else {
                    break;
                }
            }
        }
        fn string(&mut self) -> Result<String> {
            self.space();
            if self.s.get(self.pos) != Some(&b'"') {
                return Err("unsupported-token".into());
            }
            self.pos += 1;
            self.tokens += 1;
            if self.tokens > 32768 {
                return Err("token-limit".into());
            }
            let mut out = Vec::new();
            while let Some(b) = self.s.get(self.pos).copied() {
                self.pos += 1;
                match b {
                    b'"' => {
                        return String::from_utf8(out).map_err(|_| "unsupported-encoding".into());
                    }
                    b'\\' => {
                        let escaped = *self.s.get(self.pos).ok_or("truncated-escape")?;
                        self.pos += 1;
                        out.push(match escaped {
                            b'"' => b'"',
                            b'\\' => b'\\',
                            b'n' => b'\n',
                            b't' => b'\t',
                            _ => return Err("unsupported-escape".into()),
                        })
                    }
                    0 => return Err("nul-in-keyvalues".into()),
                    _ => out.push(b),
                }
            }
            Err("unterminated-string".into())
        }
        fn object(&mut self, depth: usize) -> Result<Kv> {
            if depth > 32 {
                return Err("depth-limit".into());
            }
            let mut fields = BTreeMap::new();
            loop {
                self.space();
                match self.s.get(self.pos) {
                    Some(b'}') if depth > 0 => {
                        self.pos += 1;
                        return Ok(Kv::Object(fields));
                    }
                    None if depth == 0 => return Ok(Kv::Object(fields)),
                    None => return Err("unclosed-object".into()),
                    _ => {}
                }
                let key = self.string()?.to_ascii_lowercase();
                self.space();
                let value = if self.s.get(self.pos) == Some(&b'{') {
                    self.pos += 1;
                    self.object(depth + 1)?
                } else {
                    Kv::Text(self.string()?)
                };
                if fields.insert(key, value).is_some() {
                    return Err("duplicate-key".into());
                }
            }
        }
    }
    let mut parser = Parser {
        s: source.as_bytes(),
        pos: 0,
        tokens: 0,
    };
    parser.object(0)
}
