//! Approved KOIN v1 byte framing and frozen manifests.
use crate::{Record, Result, Snapshot, Value};
use sha2::{Digest, Sha256};
pub fn hash(bytes: &[u8]) -> Vec<u8> {
    Sha256::digest(bytes).to_vec()
}
pub fn encode(v: &Value) -> Vec<u8> {
    let mut out = Vec::new();
    match v {
        Value::Null => out.push(0),
        Value::Integer(n) => {
            out.push(1);
            out.extend(n.to_be_bytes());
        }
        Value::Text(s) => {
            out.push(2);
            out.extend((s.len() as u64).to_be_bytes());
            out.extend(s.as_bytes());
        }
        Value::Blob(b) => {
            out.push(3);
            out.extend((b.len() as u64).to_be_bytes());
            out.extend(b);
        }
        Value::List(v) => {
            out.push(4);
            out.extend((v.len() as u64).to_be_bytes());
            for x in v {
                out.extend(encode(x));
            }
        }
    };
    out
}
pub fn frame(protocol: &str, payload: &Value) -> Vec<u8> {
    let mut out = b"KOIN\0\x01".to_vec();
    out.extend(encode(&protocol.into()));
    out.extend(encode(payload));
    out
}
pub fn decode(bytes: &[u8]) -> Result<Value> {
    fn take<'a>(b: &mut &'a [u8], n: usize) -> Result<&'a [u8]> {
        if b.len() < n {
            return Err("truncated canonical value".into());
        }
        let (h, t) = b.split_at(n);
        *b = t;
        Ok(h)
    }
    fn one(b: &mut &[u8], depth: usize) -> Result<Value> {
        if depth > 64 {
            return Err("canonical nesting limit".into());
        }
        let tag = take(b, 1)?[0];
        if tag == 0 {
            return Ok(Value::Null);
        }
        let raw: [u8; 8] = take(b, 8)?.try_into().map_err(|_| "length")?;
        if tag == 1 {
            return Ok(Value::Integer(i64::from_be_bytes(raw)));
        }
        let n = usize::try_from(u64::from_be_bytes(raw)).map_err(|_| "length overflow")?;
        match tag {
            2 => Ok(Value::Text(
                String::from_utf8(take(b, n)?.to_vec()).map_err(|_| "invalid UTF-8")?,
            )),
            3 => Ok(Value::Blob(take(b, n)?.to_vec())),
            4 => {
                if n > b.len() {
                    return Err("invalid count".into());
                }
                let mut v = Vec::new();
                for _ in 0..n {
                    v.push(one(b, depth + 1)?)
                }
                Ok(Value::List(v))
            }
            _ => Err("unknown canonical tag".into()),
        }
    }
    let mut rest = bytes;
    let value = one(&mut rest, 0)?;
    if !rest.is_empty() {
        return Err("trailing bytes".into());
    }
    Ok(value)
}
#[derive(Clone, Copy)]
pub struct Manifest {
    pub table: &'static str,
    pub columns: &'static [&'static str],
    pub keys: &'static [&'static str],
}
include!("manifests.rs");
pub fn bundle(snapshot: &Snapshot, protocol: &str, owner: &Value) -> Result<Value> {
    let manifest = match protocol {
        "koine.request.v1" => REQUEST,
        "koine.result.v1" => RESULT,
        "koine.decision.v1" => DECISION,
        _ => return Err("unsupported protocol".into()),
    };
    let mut tables = Vec::new();
    for m in manifest {
        let selected = |r: &&Record| -> bool {
            if protocol == "koine.decision.v1" {
                return r.get(if m.table == "reconciliation_decisions" {
                    "id"
                } else {
                    "decision_id"
                }) == owner;
            }
            if protocol == "koine.request.v1" && m.table == "collections" {
                return r.get("id") == owner;
            }
            if ["decision_claims", "supersessions", "decision_seals"].contains(&m.table) {
                return snapshot
                    .by_id("reconciliation_decisions", r.get("decision_id"))
                    .is_ok_and(|d| d.get("collection_id") == owner);
            }
            r.get("collection_id") == owner
        };
        let mut rows: Vec<_> = snapshot.rows(m.table).filter(selected).collect();
        rows.sort_by_key(|r| m.keys.iter().map(|k| encode(r.get(k))).collect::<Vec<_>>());
        for pair in rows.windows(2) {
            if m.keys.iter().all(|k| pair[0].get(k) == pair[1].get(k)) {
                return Err("duplicate canonical key".into());
            }
        }
        tables.push(Value::List(vec![
            m.table.into(),
            Value::List(m.columns.iter().map(|c| (*c).into()).collect()),
            Value::List(
                rows.iter()
                    .map(|r| Value::List(m.columns.iter().map(|c| r.get(c).clone()).collect()))
                    .collect(),
            ),
        ]));
    }
    Ok(Value::List(tables))
}
pub fn digest(snapshot: &Snapshot, protocol: &str, owner: &Value) -> Result<Vec<u8>> {
    Ok(hash(&frame(protocol, &bundle(snapshot, protocol, owner)?)))
}
pub fn authority_key(r: &Record) -> Vec<u8> {
    frame(
        "koine.authority-scope.v1",
        &Value::List(
            [
                "authority_namespace",
                "subject_context_id",
                "manager_context_id",
                "machine_context_id",
                "library_context_id",
                "provisional_token",
            ]
            .iter()
            .map(|k| r.get(k).clone())
            .collect(),
        ),
    )
}
