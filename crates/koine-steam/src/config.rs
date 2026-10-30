//! Application-owned settings, separate from immutable source evidence and schema.
//! Configuration never opens a proposed discovery root.
use crate::{fixture_fs, *};
use koine_sqlite::Store;
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
};
pub const ROOT_LIMIT: usize = 16;
const MAX_CONFIG: u64 = 128 * 1024;
/// A configured root snapshot. Fields are private so approval, binding and scope
/// change only through `Configuration` methods that persist them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Root {
    id: String,
    scope: String,
    path: PathBuf,
    layout: String,
    synthetic: bool,
    authorized: bool,
    binding: Option<String>,
}
impl Root {
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn scope(&self) -> &str {
        &self.scope
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn layout(&self) -> &str {
        &self.layout
    }
    pub fn synthetic(&self) -> bool {
        self.synthetic
    }
    pub fn authorized(&self) -> bool {
        self.authorized
    }
    pub fn binding(&self) -> Option<&str> {
        self.binding.as_deref()
    }
}
/// Read access is public; every mutation goes through `set`, `authorize`, `remove`
/// or `bind`, each of which persists before updating the in-memory roots.
#[derive(Debug)]
pub struct Configuration {
    path: PathBuf,
    owner: String,
    roots: Vec<Root>,
}
pub fn normalize(path: &Path) -> Result<PathBuf> {
    if !path.is_absolute() || fixture_fs::bytes(path).len() > 4096 {
        return Err("root must be absolute and at most 4096 bytes".into());
    }
    let mut out = PathBuf::from("/");
    let mut count = 0;
    for part in path.components() {
        match part {
            std::path::Component::RootDir | std::path::Component::CurDir => (),
            std::path::Component::Normal(name) if !name.as_encoded_bytes().contains(&0) => {
                out.push(name);
                count += 1;
            }
            _ => return Err("parent traversal or invalid root path".into()),
        }
    }
    if count == 0 || count > 128 {
        return Err("root boundary too broad or deep".into());
    }
    Ok(out)
}
pub(crate) fn uuid(value: &str) -> Result<()> {
    if value.len() != 36
        || value.bytes().enumerate().any(|(i, b)| {
            if [8, 13, 18, 23].contains(&i) {
                b != b'-'
            } else {
                !b.is_ascii_digit() && !(b'a'..=b'f').contains(&b)
            }
        })
    {
        return Err("expected lowercase UUID".into());
    }
    Ok(())
}
pub(crate) fn list(value: &Value) -> Result<&[Value]> {
    match value {
        Value::List(v) => Ok(v),
        _ => Err("expected settings list".into()),
    }
}
pub(crate) fn optional(value: &Value) -> Result<Option<String>> {
    if value == &Value::Null {
        Ok(None)
    } else {
        value.text().map(|s| Some(s.into()))
    }
}
pub(crate) fn boolean(value: &Value) -> Result<bool> {
    match value {
        Value::Integer(0) => Ok(false),
        Value::Integer(1) => Ok(true),
        _ => Err("expected boolean".into()),
    }
}
impl Configuration {
    pub fn owner(&self) -> &str {
        &self.owner
    }
    pub fn roots(&self) -> &[Root] {
        &self.roots
    }
    /// Caller must retain the Store ownership lock for this settings object's lifetime.
    pub fn open(store: &Store, database: &Path) -> Result<Self> {
        let mut path = fs::canonicalize(database)
            .map_err(|_| "database path unavailable")?
            .into_os_string();
        path.push(".steam-roots.koin");
        let owner = store
            .snapshot()?
            .rows("database_identity")
            .next()
            .ok_or("database identity missing")?
            .text("id")?
            .to_string();
        let mut out = Self {
            path: path.into(),
            owner,
            roots: Vec::new(),
        };
        match OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(&out.path)
        {
            Ok(file) => {
                let m = file.metadata().map_err(|_| "settings metadata failed")?;
                if !m.is_file() || m.permissions().mode() & 0o077 != 0 {
                    return Err("settings must be a private regular file".into());
                }
                let mut data = Vec::new();
                file.take(MAX_CONFIG + 1)
                    .read_to_end(&mut data)
                    .map_err(|_| "settings read failed")?;
                if data.len() as u64 > MAX_CONFIG {
                    return Err("settings size limit".into());
                }
                let envelope = canonical::decode(&data)?;
                let pair = list(&envelope)?;
                if pair.len() != 2
                    || pair[1].blob()? != canonical::hash(&canonical::encode(&pair[0]))
                {
                    return Err("settings checksum mismatch".into());
                }
                let value = pair[0].clone();
                let fields = list(&value)?;
                if fields.len() != 3
                    || fields[0].text()? != "koine.steam.settings.v1"
                    || fields[1].text()? != out.owner
                {
                    return Err("unsupported settings or different database".into());
                }
                for item in list(&fields[2])? {
                    let f = list(item)?;
                    if f.len() != 8 {
                        return Err("unsupported settings root".into());
                    }
                    let root = Root {
                        id: f[0].text()?.into(),
                        scope: f[1].text()?.into(),
                        path: fixture_fs::from_bytes(f[2].blob()?.into()),
                        layout: f[3].text()?.into(),
                        synthetic: boolean(&f[4])?,
                        authorized: boolean(&f[5])?,
                        binding: optional(&f[6])?,
                    };
                    if f[7].text()? != "manifests-and-index-only" {
                        return Err("unsupported authorization categories".into());
                    }
                    uuid(&root.id)?;
                    uuid(&root.scope)?;
                    if normalize(&root.path)? != root.path
                        || out
                            .roots
                            .iter()
                            .any(|r| r.id == root.id || r.path == root.path)
                    {
                        return Err("invalid or duplicate configured root".into());
                    }
                    out.roots.push(root);
                }
                if out.roots.len() > ROOT_LIMIT {
                    return Err("configured root limit".into());
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
            Err(_) => return Err("settings unavailable (symlinks refused)".into()),
        }
        Ok(out)
    }
    pub fn set(&mut self, root_id: &str, path: &Path, layout: &str, synthetic: bool) -> Result<()> {
        uuid(root_id)?;
        if layout.is_empty() || layout.len() > 64 {
            return Err("invalid layout label".into());
        }
        let path = normalize(path)?;
        if self.roots.iter().any(|r| r.id != root_id && r.path == path) {
            return Err("root already configured".into());
        }
        if let Some(r) = self.roots.iter().find(|r| r.id == root_id) {
            if r.path == path && r.layout == layout && r.synthetic == synthetic {
                return Ok(());
            }
        } else if self.roots.len() == ROOT_LIMIT {
            return Err("configured root limit".into());
        }
        let root = Root {
            id: root_id.into(),
            scope: id(),
            path,
            layout: layout.into(),
            synthetic,
            authorized: false,
            binding: None,
        };
        let mut roots = self.roots.clone();
        roots.retain(|r| r.id != root_id);
        roots.push(root);
        roots.sort_by(|a, b| a.id.cmp(&b.id));
        self.persist(roots)
    }
    /// Explicit approval for these exact path bytes and the fixed read categories.
    pub fn authorize(&mut self, root_id: &str, exact_path: &Path) -> Result<()> {
        let mut roots = self.roots.clone();
        let root = roots
            .iter_mut()
            .find(|r| r.id == root_id)
            .ok_or("unknown root")?;
        if root.path != exact_path || fixture_fs::bytes(&root.path) != fixture_fs::bytes(exact_path)
        {
            return Err("authorization must name the exact configured path".into());
        }
        root.authorized = true;
        self.persist(roots)
    }
    pub fn remove(&mut self, root_id: &str) -> Result<()> {
        if !self.roots.iter().any(|r| r.id == root_id) {
            return Err("unknown root".into());
        }
        let roots = self
            .roots
            .iter()
            .filter(|r| r.id != root_id)
            .cloned()
            .collect();
        self.persist(roots)
    }
    pub(crate) fn bind(&mut self, scope: &str, identity: &str) -> Result<()> {
        if self
            .roots
            .iter()
            .any(|r| r.scope != scope && r.binding.as_deref() == Some(identity))
        {
            return Err("duplicate-physical-scope".into());
        }
        let mut roots = self.roots.clone();
        let r = roots
            .iter_mut()
            .find(|r| r.scope == scope)
            .ok_or(crate::controlled::CHANGED_DURING_DISCOVERY)?;
        if r.binding.as_deref().is_some_and(|b| b != identity) {
            return Err("root-identity-changed-reconfigure-required".into());
        }
        if r.binding.is_some() {
            return Ok(());
        }
        r.binding = Some(identity.into());
        self.persist(roots)
    }
    fn persist(&mut self, roots: Vec<Root>) -> Result<()> {
        let value = Value::List(vec![
            "koine.steam.settings.v1".into(),
            self.owner.clone().into(),
            Value::List(
                roots
                    .iter()
                    .map(|r| {
                        Value::List(vec![
                            r.id.clone().into(),
                            r.scope.clone().into(),
                            fixture_fs::bytes(&r.path).into(),
                            r.layout.clone().into(),
                            i64::from(r.synthetic).into(),
                            i64::from(r.authorized).into(),
                            r.binding.clone().map_or(Value::Null, Value::Text),
                            "manifests-and-index-only".into(),
                        ])
                    })
                    .collect(),
            ),
        ]);
        let digest = canonical::hash(&canonical::encode(&value));
        let data = canonical::encode(&Value::List(vec![value, digest.into()]));
        let temp = self.path.with_extension(format!("{}.partial", id()));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temp)
            .map_err(|_| "settings staging failed")?;
        file.write_all(&data)
            .and_then(|()| file.sync_all())
            .map_err(|_| "settings sync failed")?;
        fs::rename(&temp, &self.path).map_err(|_| "settings finalization failed")?;
        File::open(self.path.parent().ok_or("settings parent")?)
            .and_then(|f| f.sync_all())
            .map_err(|_| "settings directory sync failed; reopen settings before retry")?;
        self.roots = roots;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn established_physical_aliases_are_refused_even_across_batches() {
        let lab = crate::test_support::Lab::new();
        let a = lab.add("a");
        let b = lab.add("b");
        let a = lab.root(&a);
        let b = lab.root(&b);
        let mut c = lab.settings.lock().unwrap();
        c.bind(&a.scope, "synthetic-physical-id").unwrap();
        assert_eq!(
            c.bind(&b.scope, "synthetic-physical-id").unwrap_err(),
            "duplicate-physical-scope"
        );
    }
    #[test]
    fn binding_a_scope_no_longer_configured_reports_change_during_discovery() {
        let lab = crate::test_support::Lab::new();
        let removed = lab.add("removed");
        let reconfigured = lab.add("reconfigured");
        let removed = lab.root(&removed);
        let old = lab.root(&reconfigured);
        let mut c = lab.settings.lock().unwrap();
        c.remove(removed.id()).unwrap();
        c.set(old.id(), old.path(), "flatpak", true).unwrap();
        for scope in [removed.scope(), old.scope()] {
            assert_eq!(
                c.bind(scope, "synthetic-physical-id").unwrap_err(),
                "configuration-changed-during-discovery"
            );
        }
        assert!(c.roots().iter().all(|r| r.binding().is_none()));
    }
    #[test]
    fn corrupt_settings_and_settings_symlinks_fail_closed() {
        let lab = crate::test_support::Lab::new();
        let _root = lab.add("a");
        let path = lab.settings.lock().unwrap().path.clone();
        let mut bytes = fs::read(&path).unwrap();
        let n = bytes.len() - 1;
        bytes[n] ^= 1;
        fs::write(&path, bytes).unwrap();
        assert!(
            Configuration::open(&lab.store, &lab.db)
                .err()
                .unwrap()
                .contains("checksum")
        );
        fs::remove_file(&path).unwrap();
        let other = lab.dir.path().join("not-settings");
        fs::write(&other, b"unchanged").unwrap();
        std::os::unix::fs::symlink(&other, &path).unwrap();
        assert!(Configuration::open(&lab.store, &lab.db).is_err());
        assert_eq!(fs::read(other).unwrap(), b"unchanged");
    }
}
