//! Linux descriptor-relative, no-follow access to an explicit marked fixture.
use koine_core::Result;
use std::{
    ffi::CString,
    fs::File,
    io::{Read, Seek, SeekFrom},
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::{
            ffi::{OsStrExt, OsStringExt},
            fs::{MetadataExt, PermissionsExt},
        },
    },
    path::{Component, Path, PathBuf},
};
pub struct FixtureFs {
    pub(crate) root: File,
    pub(crate) path: PathBuf,
    pub id: String,
    pub(crate) confined: bool,
}
fn child(dir: &File, name: &std::ffi::OsStr, directory: bool, confined: bool) -> Result<File> {
    let name = CString::new(name.as_bytes()).map_err(|_| "unsafe-path")?;
    let flags = libc::O_RDONLY
        | libc::O_CLOEXEC
        | libc::O_NOFOLLOW
        | libc::O_NONBLOCK
        | if directory { libc::O_DIRECTORY } else { 0 };
    // SAFETY: parent descriptor and CString are live; openat returns a new owned fd.
    let fd = if confined {
        #[repr(C)]
        struct OpenHow {
            flags: u64,
            mode: u64,
            resolve: u64,
        }
        // Linux openat2: NO_XDEV | NO_MAGICLINKS | NO_SYMLINKS | BENEATH.
        let how = OpenHow {
            flags: flags as u64,
            mode: 0,
            resolve: 0x01 | 0x02 | 0x04 | 0x08,
        };
        // SAFETY: valid descriptor, NUL-terminated name and sized open_how input.
        unsafe {
            libc::syscall(
                libc::SYS_openat2,
                dir.as_raw_fd(),
                name.as_ptr(),
                &how,
                std::mem::size_of::<OpenHow>(),
            ) as i32
        }
    } else {
        // SAFETY: valid directory descriptor and NUL-terminated name.
        unsafe { libc::openat(dir.as_raw_fd(), name.as_ptr(), flags) }
    };
    if fd < 0 {
        return Err(match std::io::Error::last_os_error().raw_os_error() {
            Some(libc::ENOENT) => "missing-path",
            Some(libc::EACCES) => "unreadable-path",
            Some(libc::EPERM) => "inaccessible-path",
            Some(libc::ENOTDIR) => "unsupported-directory-or-symlink",
            Some(libc::EXDEV) => "mount-boundary-refused",
            Some(libc::ENOSYS | libc::EINVAL) => "unsupported-confinement",
            Some(libc::ESTALE | libc::EAGAIN | libc::ENODEV | libc::EIO) => {
                "temporarily-unavailable-path"
            }
            Some(libc::ELOOP) => "symlink-refused",
            _ => "unavailable-path",
        }
        .into());
    }
    // SAFETY: successful openat uniquely transfers this descriptor into File.
    let file = unsafe { File::from_raw_fd(fd) };
    let meta = file.metadata().map_err(|_| "metadata-failed")?;
    if directory && !meta.is_dir() || !directory && !meta.is_file() {
        return Err("special-file-refused".into());
    }
    if meta.permissions().mode() & if directory { 0o555 } else { 0o444 } == 0 {
        return Err("unreadable-path".into());
    }
    Ok(file)
}
pub fn safe_relative(path: &Path) -> bool {
    path.components()
        .all(|c| matches!(c, Component::Normal(_) | Component::CurDir))
        && !path.as_os_str().as_bytes().contains(&0)
}
impl FixtureFs {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if !path.is_absolute() {
            return Err("fixture path must be explicit and absolute".into());
        }
        let mut fs = Self::boundary(path, String::new())?;
        fs.confined = false; // Preserve the frozen fixture-v1 filesystem policy.
        let marker = fs.read(Path::new(".koine-synthetic-root"))?;
        let marker = std::str::from_utf8(&marker).map_err(|_| "invalid fixture marker")?;
        let mut lines = marker.lines();
        if lines.next() != Some("koine-fixture-v1") {
            return Err("synthetic fixture marker required".into());
        }
        let id = lines.next().ok_or("fixture UUID missing")?;
        if id.len() != 36
            || id.as_bytes().get(14) != Some(&b'4')
            || !id.as_bytes().get(19).is_some_and(|b| b"89ab".contains(b))
            || id.bytes().enumerate().any(|(i, b)| {
                if [8, 13, 18, 23].contains(&i) {
                    b != b'-'
                } else {
                    !b.is_ascii_digit() && !(b'a'..=b'f').contains(&b)
                }
            })
            || lines.next().is_some()
        {
            return Err("invalid fixture UUID".into());
        }
        fs.id = id.into();
        Ok(fs)
    }
    /// Opens only the explicitly named directory. No canonicalization or symlink following.
    pub(crate) fn boundary(path: &Path, id: String) -> Result<Self> {
        let path = crate::config::normalize(path)?;
        let mut root = File::open("/").map_err(|_| "root unavailable")?;
        for c in path.components() {
            if let Component::Normal(name) = c {
                root = child(&root, name, true, false)?;
            }
        }
        Ok(Self {
            root,
            path,
            id,
            confined: true,
        })
    }
    pub(crate) fn identity(&self) -> Result<String> {
        let m = self.root.metadata().map_err(|_| "metadata-failed")?;
        let birth = m
            .created()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok());
        let birth = birth.map_or_else(
            || "unknown-birth".into(),
            |b| format!("{}:{}", b.as_secs(), b.subsec_nanos()),
        );
        Ok(format!(
            "koine.fs-identity.v1:{}:{}:{birth}",
            m.dev(),
            m.ino()
        ))
    }
    fn directory(&self, path: &Path) -> Result<File> {
        if !safe_relative(path) {
            return Err("unsafe-path".into());
        }
        let mut dir = self
            .root
            .try_clone()
            .map_err(|_| "descriptor-clone-failed")?;
        for c in path.components() {
            if let Component::Normal(name) = c {
                dir = child(&dir, name, true, self.confined)?
            }
        }
        Ok(dir)
    }
    pub fn read(&self, path: &Path) -> Result<Vec<u8>> {
        if !safe_relative(path) {
            return Err("unsafe-path".into());
        }
        let name = path.file_name().ok_or("missing filename")?;
        let parent = self.directory(path.parent().unwrap_or(Path::new("")))?;
        let mut file = child(&parent, name, false, self.confined)?;
        let before = file.metadata().map_err(|_| "metadata-failed")?;
        if before.len() > 1_048_576 {
            return Err("oversized-file".into());
        }
        let mut bytes = Vec::new();
        file.by_ref()
            .take(1_048_577)
            .read_to_end(&mut bytes)
            .map_err(|_| "read-failed")?;
        file.seek(SeekFrom::Start(0)).map_err(|_| "seek-failed")?;
        let mut second = Vec::new();
        file.by_ref()
            .take(1_048_577)
            .read_to_end(&mut second)
            .map_err(|_| "read-failed")?;
        let after = file.metadata().map_err(|_| "metadata-failed")?;
        if bytes.len() > 1_048_576
            || bytes != second
            || before.len() != after.len()
            || (before.mtime(), before.mtime_nsec()) != (after.mtime(), after.mtime_nsec())
        {
            return Err("changed-during-read".into());
        }
        Ok(bytes)
    }
    pub fn list(&self, path: &Path) -> Result<Vec<std::ffi::OsString>> {
        let dir = self.directory(path)?;
        let entries = std::fs::read_dir(format!("/proc/self/fd/{}", dir.as_raw_fd()))
            .map_err(|_| "enumeration-failed")?;
        let mut out = Vec::new();
        for entry in entries {
            out.push(entry.map_err(|_| "enumeration-failed")?.file_name());
            if out.len() > 8192 {
                return Err("directory-entry-limit".into());
            }
        }
        out.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
        Ok(out)
    }
    pub fn library_path(&self, path: &str) -> Result<PathBuf> {
        let path = Path::new(path);
        let relative = path
            .strip_prefix(&self.path)
            .map_err(|_| "library-outside-fixture")?;
        if !safe_relative(relative) {
            return Err("unsafe-library-path".into());
        }
        Ok(relative.into())
    }
}
pub fn bytes(path: &Path) -> Vec<u8> {
    path.as_os_str().as_bytes().to_vec()
}
pub fn from_bytes(bytes: Vec<u8>) -> PathBuf {
    std::ffi::OsString::from_vec(bytes).into()
}
