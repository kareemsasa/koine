//! Finite catalog, exact compatibility, record I/O and canonical preparation.
use super::*;

pub(super) fn check_identity(c: &Connection) -> Result<()> {
    let app: i64 = sql(c.query_row("PRAGMA application_id", [], |r| r.get(0)))?;
    let version: i64 = sql(c.query_row("PRAGMA user_version", [], |r| r.get(0)))?;
    if app != APPLICATION_ID || version != VERSION {
        return Err(
            "unsupported database identity or schema; no automatic upgrade/recreation".into(),
        );
    }
    // Compare the complete expected catalog, not merely the version integer.
    let expected = sql(Connection::open_in_memory())?;
    sql(expected.execute_batch(BOOTSTRAP))?;
    fn catalog(c: &Connection) -> Result<Vec<(String, String, String)>> {
        let mut stmt=sql(c.prepare("SELECT type,name,sql FROM sqlite_schema WHERE sql IS NOT NULL AND name NOT LIKE 'sqlite_%' ORDER BY type,name"))?;
        sql(stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))))?
            .map(sql)
            .collect()
    }
    if catalog(c)? != catalog(&expected)? {
        return Err("catalog differs from bootstrap manifest".into());
    }
    let identity_count:i64=sql(c.query_row("SELECT count(*) FROM database_identity WHERE family='koine.offline.foundation' AND initial_version=1",[],|r|r.get(0)))?;
    let histories:i64=sql(c.query_row("SELECT count(*) FROM migration_history WHERE migration_id='bootstrap.v1' AND artifact_digest=?1",[canonical::hash(BOOTSTRAP.as_bytes())],|r|r.get(0)))?;
    if identity_count != 1 || histories != 1 {
        return Err("invalid database identity/history".into());
    }
    Ok(())
}
pub(super) fn integrity(c: &Connection) -> Result<()> {
    let mut st = sql(c.prepare("PRAGMA integrity_check"))?;
    let results: Vec<String> = sql(st.query_map([], |r| r.get(0)))?
        .map(sql)
        .collect::<Result<_>>()?;
    if results != ["ok"] {
        return Err(format!("integrity_check: {results:?}"));
    }
    let mut st = sql(c.prepare("PRAGMA foreign_key_check"))?;
    if sql(st.query([]))?
        .next()
        .map_err(|e| e.to_string())?
        .is_some()
    {
        return Err("foreign_key_check failed".into());
    }
    Ok(())
}
pub(super) fn load(c: &Connection) -> Result<Snapshot> {
    let mut records = Vec::new();
    for m in CATALOG {
        let query = format!(
            "SELECT {} FROM {} ORDER BY {}",
            m.columns.join(","),
            m.table,
            m.keys.join(",")
        );
        let mut stmt = sql(c.prepare(&query))?;
        let mut rows = sql(stmt.query([]))?;
        while let Some(row) = sql(rows.next())? {
            let mut fields = std::collections::BTreeMap::new();
            for (i, col) in m.columns.iter().enumerate() {
                let v = match sql(row.get_ref(i))? {
                    ValueRef::Null => Value::Null,
                    ValueRef::Integer(n) => Value::Integer(n),
                    ValueRef::Text(t) => Value::Text(
                        std::str::from_utf8(t)
                            .map_err(|_| "invalid UTF-8 in stored TEXT")?
                            .into(),
                    ),
                    ValueRef::Blob(b) => Value::Blob(b.to_vec()),
                    ValueRef::Real(_) => return Err("REAL forbidden in evidence".into()),
                };
                fields.insert((*col).into(), v);
            }
            records.push(Record {
                table: m.table.into(),
                fields,
            });
        }
    }
    Ok(Snapshot { records })
}
pub(super) fn read_snapshot(c: &Connection) -> Result<Snapshot> {
    deadline();
    sql(c.execute_batch("BEGIN"))?;
    let result = load(c);
    let end = sql(c.execute_batch("ROLLBACK"));
    end?;
    result
}
pub(super) fn normalized(mut r: Record) -> Result<Record> {
    let m = CATALOG
        .iter()
        .find(|m| m.table == r.table)
        .ok_or("unknown record family")?;
    if r.fields.keys().any(|k| !m.columns.contains(&k.as_str())) {
        return Err("unknown record column".into());
    }
    for col in m.columns {
        r.fields.entry((*col).into()).or_insert(Value::Null);
    }
    Ok(r)
}
pub(super) fn insert(c: &Connection, r: &Record) -> Result<()> {
    let m = CATALOG
        .iter()
        .find(|m| m.table == r.table)
        .ok_or("unknown table")?;
    let values: Result<Vec<SqlValue>> = m
        .columns
        .iter()
        .map(|col| match r.get(col) {
            Value::Null => Ok(SqlValue::Null),
            Value::Integer(n) => Ok(SqlValue::Integer(*n)),
            Value::Text(s) => Ok(SqlValue::Text(s.clone())),
            Value::Blob(b) => Ok(SqlValue::Blob(b.clone())),
            Value::List(_) => Err("lists must be canonically encoded as bytes".into()),
        })
        .collect();
    let query = format!(
        "INSERT INTO {} ({}) VALUES ({})",
        m.table,
        m.columns.join(","),
        vec!["?"; m.columns.len()].join(",")
    );
    sql(c.execute(&query, params_from_iter(values?)))?;
    Ok(())
}
/// Fill only new, explicitly unfinished digest fields; never reseal durable records.
pub(super) fn prepare(existing: &Snapshot, rows: Vec<Record>) -> Result<Vec<Record>> {
    let mut rows: Vec<_> = rows.into_iter().map(normalized).collect::<Result<_>>()?;
    if rows
        .iter()
        .any(|r| ["database_identity", "migration_history"].contains(&r.table.as_str()))
    {
        return Err("metadata is bootstrap-only".into());
    }
    let mut proposed = existing.clone();
    for r in &mut rows {
        let m = CATALOG
            .iter()
            .find(|m| m.table == r.table)
            .ok_or("unknown table")?;
        if let Some(old) = existing
            .rows(&r.table)
            .find(|old| m.keys.iter().all(|k| old.get(k) == r.get(k)))
        {
            let digest_field = match r.table.as_str() {
                "collections" => Some("request_digest"),
                "decision_seals" => Some("content_digest"),
                "collection_closures" => Some("result_digest"),
                _ => None,
            };
            if let Some(field) = digest_field
                && r.get(field) == &Value::Null
            {
                r.set(field, old.get(field).clone())
            }
            if old != r {
                return Err("conflicting immutable redelivery".into());
            }
        } else {
            proposed.records.push(r.clone())
        }
    }
    let new_claims = rows
        .iter()
        .filter(|r| r.table == "claims" && existing.by_id("claims", r.get("id")).is_err())
        .map(|r| r.get("id").clone())
        .collect();
    let planned = reconcile::plan(&proposed, &new_claims)?;
    for r in planned {
        let r = normalized(r)?;
        proposed.records.push(r.clone());
        rows.push(r)
    }
    let new_collections: Vec<_> = rows
        .iter()
        .filter(|r| r.table == "collections")
        .map(|r| r.get("id").clone())
        .collect();
    if rows.iter().any(|r| {
        r.table != "requested_units"
            && r.table != "collections"
            && r.get("collection_id") != &Value::Null
            && new_collections.contains(r.get("collection_id"))
    }) {
        return Err("register request in a separate commit before result application".into());
    }
    for (table, owner, protocol, field) in [
        ("collections", "id", "koine.request.v1", "request_digest"),
        (
            "decision_seals",
            "decision_id",
            "koine.decision.v1",
            "content_digest",
        ),
        (
            "collection_closures",
            "collection_id",
            "koine.result.v1",
            "result_digest",
        ),
    ] {
        for r in rows.iter_mut().filter(|r| r.table == table) {
            if r.get(field) == &Value::Null {
                if table == "decision_seals" {
                    let decision =
                        proposed.by_id("reconciliation_decisions", r.get("decision_id"))?;
                    reconcile::validate_decision_content(&proposed, decision)?;
                }
                let digest = canonical::digest(&proposed, protocol, r.get(owner))?;
                r.set(field, digest);
                if let Some(p) = proposed
                    .records
                    .iter_mut()
                    .find(|p| p.table == table && p.get(owner) == r.get(owner))
                {
                    *p = r.clone()
                }
            }
        }
    }
    for r in &rows {
        if !["collections", "requested_units"].contains(&r.table.as_str())
            && r.get("collection_id") != &Value::Null
            && !proposed
                .rows("collection_closures")
                .any(|c| c.get("collection_id") == r.get("collection_id"))
        {
            return Err("result application requires atomic closure".into());
        }
    }
    rows.sort_by_key(|r| match r.table.as_str() {
        "collection_closures" => 2,
        "decision_seals" => 1,
        _ => 0,
    });
    Ok(rows)
}
