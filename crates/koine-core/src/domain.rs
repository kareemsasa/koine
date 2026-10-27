use std::collections::BTreeMap;
use uuid::Uuid;

pub type Result<T> = std::result::Result<T, String>;
pub fn id() -> String {
    Uuid::new_v4().to_string()
}

/// Exact logical values; REAL is deliberately unrepresentable.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Value {
    Null,
    Integer(i64),
    Text(String),
    Blob(Vec<u8>),
    List(Vec<Value>),
}
impl From<&str> for Value {
    fn from(v: &str) -> Self {
        Self::Text(v.into())
    }
}
impl From<String> for Value {
    fn from(v: String) -> Self {
        Self::Text(v)
    }
}
impl From<i64> for Value {
    fn from(v: i64) -> Self {
        Self::Integer(v)
    }
}
impl From<Vec<u8>> for Value {
    fn from(v: Vec<u8>) -> Self {
        Self::Blob(v)
    }
}
impl Value {
    pub fn text(&self) -> Result<&str> {
        if let Self::Text(v) = self {
            Ok(v)
        } else {
            Err("expected text".into())
        }
    }
    pub fn integer(&self) -> Result<i64> {
        if let Self::Integer(v) = self {
            Ok(*v)
        } else {
            Err("expected integer".into())
        }
    }
    pub fn blob(&self) -> Result<&[u8]> {
        if let Self::Blob(v) = self {
            Ok(v)
        } else {
            Err("expected bytes".into())
        }
    }
}

/// Finite, normalized evidence records. Fields are validated against the catalog;
/// they are not an extensible attributes store or an authoritative current-state cache.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Record {
    pub table: String,
    pub fields: BTreeMap<String, Value>,
}
impl Record {
    pub fn new(table: &str, fields: impl IntoIterator<Item = (impl Into<String>, Value)>) -> Self {
        Self {
            table: table.into(),
            fields: fields.into_iter().map(|(k, v)| (k.into(), v)).collect(),
        }
    }
    pub fn get(&self, key: &str) -> &Value {
        self.fields.get(key).unwrap_or(&Value::Null)
    }
    pub fn text(&self, key: &str) -> Result<&str> {
        self.get(key)
            .text()
            .map_err(|e| format!("{}.{}: {e}", self.table, key))
    }
    pub fn int(&self, key: &str) -> Result<i64> {
        self.get(key).integer()
    }
    pub fn is(&self, key: &str, value: &str) -> bool {
        self.get(key) == &Value::from(value)
    }
    pub fn set(&mut self, key: &str, value: impl Into<Value>) {
        self.fields.insert(key.into(), value.into());
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Snapshot {
    pub records: Vec<Record>,
}
impl Snapshot {
    pub fn rows<'a>(&'a self, table: &str) -> impl Iterator<Item = &'a Record> + 'a {
        let table = table.to_string();
        self.records.iter().filter(move |r| r.table == table)
    }
    pub fn find(&self, table: &str, key: &str, value: &Value) -> Result<&Record> {
        self.rows(table)
            .find(|r| r.get(key) == value)
            .ok_or_else(|| format!("missing {table}.{key}"))
    }
    pub fn by_id(&self, table: &str, id: &Value) -> Result<&Record> {
        self.find(table, "id", id)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Knowledge {
    Observed(Value),
    ConfirmedNegative(Value),
    Unknown,
    Disputed,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Freshness {
    Current,
    Stale,
    Unknown,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Conclusion {
    pub subject: String,
    pub dimension: String,
    pub authority: String,
    pub knowledge: Knowledge,
    pub freshness: Freshness,
    pub supporting_claims: Vec<String>,
    pub last_known_claims: Vec<String>,
    pub diagnostics: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Handoff {
    NotRequested,
    Pending,
    Accepted,
    Failed,
    Inconclusive,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Process {
    Unobserved,
    Observed,
    Inconclusive,
    Exited,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttemptConclusion {
    pub attempt: String,
    pub target: String,
    pub handoff: Handoff,
    pub process: Process,
    pub exit_status: Option<i64>,
    pub issuance: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Assessment {
    pub at: i64,
    pub observer_epoch: Option<String>,
    pub reconciliation_rule: String,
    pub freshness_rule: String,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Library {
    pub claims: Vec<Conclusion>,
    pub attempts: Vec<AttemptConclusion>,
    pub diagnostics: Vec<String>,
    pub identity_components: Vec<IdentityComponent>,
    pub collections: Vec<CollectionConclusion>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IdentityComponent {
    pub handles: Vec<String>,
    pub disputed: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CollectionConclusion {
    pub collection: String,
    pub outcome: String,
    pub limitations: Vec<String>,
}

/// Transports receive only a target handle and launcher context. There is no shell API.
pub trait Transport {
    fn submit(&mut self, target: &str, launcher_context: &str) -> Result<TransportResult>;
}
#[derive(Clone, Debug)]
pub struct TransportResult {
    pub issuance: String,
    pub disposition: String,
    pub status: Option<i64>,
    pub boundary: String,
    pub reason: Option<String>,
}
