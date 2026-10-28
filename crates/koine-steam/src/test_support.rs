use crate::config::{Configuration, Root};
use crate::*;
use koine_sqlite::Store;
use std::{fs, sync::Mutex};
pub(crate) struct Lab {
    pub dir: tempfile::TempDir,
    pub db: PathBuf,
    pub store: Arc<Store>,
    pub settings: Arc<Mutex<Configuration>>,
}
impl Lab {
    pub fn new() -> Self {
        Self::with_store(false)
    }
    fn with_store(fault: bool) -> Self {
        let _ = fault;
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("db.sqlite");
        #[cfg(feature = "fault-injection")]
        let store = if fault {
            Store::create_fault_lab(&db)
        } else {
            Store::create(&db)
        }
        .unwrap();
        #[cfg(not(feature = "fault-injection"))]
        let store = Store::create(&db).unwrap();
        let store = Arc::new(store);
        let settings = Arc::new(Mutex::new(Configuration::open(&store, &db).unwrap()));
        Self {
            dir,
            db,
            store,
            settings,
        }
    }
    #[cfg(feature = "fault-injection")]
    pub fn fault() -> Self {
        Self::with_store(true)
    }
    pub fn add(&self, name: &str) -> String {
        let path = self.library(name);
        fs::write(
            path.join(".koine-synthetic-root"),
            format!("koine-fixture-v1\n{}\n", id()),
        )
        .unwrap();
        self.configure(&path, true)
    }
    /// Real mode over an unmarked synthetic directory, approved for its exact path.
    pub fn add_authorized_real(&self, name: &str) -> String {
        let path = self.library(name);
        let rid = self.configure(&path, false);
        self.settings
            .lock()
            .unwrap()
            .authorize(&rid, &path)
            .unwrap();
        rid
    }
    fn library(&self, name: &str) -> PathBuf {
        let path = self.dir.path().join(name);
        fs::create_dir_all(path.join("steamapps")).unwrap();
        fs::write(
            path.join("steamapps/libraryfolders.vdf"),
            format!(
                "\"libraryfolders\" {{\"0\" {{\"path\" \"{}\"}}}}",
                path.display()
            ),
        )
        .unwrap();
        for app in [41, 42] {
            fs::write(path.join(format!("steamapps/appmanifest_{app}.acf")),format!("\"AppState\" {{\"appid\" \"{app}\" \"installdir\" \"Synthetic {app}\" \"name\" \"Synthetic {app}\" \"StateFlags\" \"4\"}}")).unwrap();
        }
        path
    }
    fn configure(&self, path: &Path, synthetic: bool) -> String {
        let rid = id();
        self.settings
            .lock()
            .unwrap()
            .set(&rid, path, "native", synthetic)
            .unwrap();
        rid
    }
    pub fn root(&self, rid: &str) -> Root {
        self.settings
            .lock()
            .unwrap()
            .roots()
            .iter()
            .find(|r| r.id() == rid)
            .unwrap()
            .clone()
    }
    pub fn report(&self, rid: &str) -> Report {
        let root = self.root(rid);
        let plan =
            controlled::plan(self.settings.lock().unwrap().owner(), &root, id(), 10).unwrap();
        integration::register(&self.store, &plan).unwrap();
        controlled::scan(
            &self.settings,
            &root,
            plan,
            Arc::new(AtomicBool::new(false)),
            &mut std::collections::BTreeSet::new(),
            &mut |_| {},
        )
        .unwrap()
    }
}
