use koine_core::{canonical::*, *};
fn hex(b: &[u8]) -> String {
    b.iter().map(|n| format!("{n:02x}")).collect()
}
#[test]
fn independent_framing_and_sha256_vectors() {
    let values = [
        Value::Null,
        Value::Integer(-1),
        "Koiné".into(),
        vec![0xff, 0x80].into(),
        Value::List(vec![]),
        Value::List(vec![
            Value::Null,
            Value::Integer(i64::MIN),
            "".into(),
            vec![].into(),
            Value::List(vec![1i64.into(), "x".into()]),
        ]),
    ];
    for (line, v) in include_str!("fixtures/canonical-v1.tsv")
        .lines()
        .filter(|l| !l.starts_with('#'))
        .zip(values)
    {
        let fields: Vec<_> = line.split('\t').collect();
        assert_eq!(hex(&encode(&v)), fields[1]);
        assert_eq!(hex(&frame("koine.vector.v1", &v)), fields[2]);
        assert_eq!(hex(&hash(&frame("koine.vector.v1", &v))), fields[3]);
        assert_eq!(decode(&encode(&v)).unwrap(), v);
    }
    assert_eq!(
        hex(&hash(b"abc")),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}
#[test]
fn invalid_and_ambiguous_encodings_rejected() {
    assert_ne!(encode(&Value::Null), encode(&"".into()));
    assert_ne!(encode(&vec![].into()), encode(&Value::List(vec![])));
    for invalid in [
        vec![],
        vec![5, 0, 0, 0, 0, 0, 0, 0, 0],
        vec![0, 0],
        vec![2, 0, 0, 0, 0, 0, 0, 0, 1, 0xff],
        vec![4, 0, 0, 0, 0, 0, 0, 0, 2, 0],
        vec![1, 0],
    ] {
        assert!(decode(&invalid).is_err());
    }
}
#[test]
fn frozen_membership_order_and_duplicate_keys() {
    let owner: Value = "00000000-0000-4000-8000-000000000001".into();
    let d = Record::new(
        "reconciliation_decisions",
        [
            ("id", owner.clone()),
            ("kind", "dispute".into()),
            ("rule_id", owner.clone()),
            ("reason", "synthetic".into()),
            ("assessed_at", 1i64.into()),
        ],
    );
    let a = Record::new(
        "decision_claims",
        [
            ("decision_id", owner.clone()),
            ("claim_id", "00000000-0000-4000-8000-000000000002".into()),
            ("role", "conflicting".into()),
        ],
    );
    let b = Record::new(
        "decision_claims",
        [
            ("decision_id", owner.clone()),
            ("claim_id", "00000000-0000-4000-8000-000000000003".into()),
            ("role", "conflicting".into()),
        ],
    );
    let mut s = Snapshot {
        records: vec![d, a, b],
    };
    let original = digest(&s, "koine.decision.v1", &owner).unwrap();
    s.records.reverse();
    assert_eq!(original, digest(&s, "koine.decision.v1", &owner).unwrap());
    s.records
        .push(Record::new("evidence_records", [("id", id().into())]));
    assert_eq!(original, digest(&s, "koine.decision.v1", &owner).unwrap());
    s.records.push(s.records[0].clone());
    assert!(digest(&s, "koine.decision.v1", &owner).is_err());
    assert!(digest(&s, "unknown", &owner).is_err());
}

#[test]
fn independent_request_result_and_decision_bundle_vectors() {
    use std::collections::BTreeMap;
    fn unhex(s: &str) -> Vec<u8> {
        s.as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    }
    let mut records: BTreeMap<(String, String), Record> = BTreeMap::new();
    for line in include_str!("fixtures/bundle-records.tsv").lines() {
        let fields: Vec<_> = line.split('\t').collect();
        let value = match fields[3] {
            "N" => Value::Null,
            "I" => Value::Integer(fields[4].parse().unwrap()),
            "B" => Value::Blob(unhex(fields[4])),
            "T" => fields[4].into(),
            _ => panic!("bad reference vector"),
        };
        records
            .entry((fields[0].into(), fields[1].into()))
            .or_insert_with(|| Record::new(fields[0], Vec::<(String, Value)>::new()))
            .set(fields[2], value);
    }
    let snapshot = Snapshot {
        records: records.into_values().collect(),
    };
    for line in include_str!("fixtures/bundles-v1.tsv").lines() {
        let fields: Vec<_> = line.split('\t').collect();
        let protocol = fields[0];
        let owner: Value = fields[1].into();
        assert_eq!(
            hex(&frame(
                protocol,
                &bundle(&snapshot, protocol, &owner).unwrap()
            )),
            fields[2]
        );
        assert_eq!(
            hex(&digest(&snapshot, protocol, &owner).unwrap()),
            fields[3]
        );
    }
}
