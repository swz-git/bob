// custom diffing tool for directories powered by zstd

use anyhow::{Context, anyhow};
use log::{error, info};
use rapidhash::v3::rapidhash_v3;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use rkyv::with::AsString;
use rkyv::{Archive, Deserialize, Serialize, rancor};
use std::fs::{self};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::thread::available_parallelism;
use std::time::Instant;
use zstd::dict::DDict;
use zstd::zstd_safe::CParameter;

#[cfg(target_family = "unix")]
use std::os::unix::fs::PermissionsExt;

#[derive(Debug, PartialEq, Archive, Deserialize, Serialize)]
struct FileFlags {
    /// Only applies to linux
    executable: bool,
}

#[derive(Debug, PartialEq, Archive, Deserialize, Serialize)]
enum DataState {
    Identical,
    Patch(Box<[u8]>),
    Raw(Box<[u8]>),
}

#[derive(Debug, PartialEq, Archive, Deserialize, Serialize)]
enum DirDiffEntry {
    File {
        #[rkyv(with = AsString)]
        path: PathBuf,
        state: DataState,
        flags: Option<FileFlags>,
    },
    Dir(#[rkyv(with = AsString)] PathBuf),
}

#[derive(Debug, PartialEq, Archive, Deserialize, Serialize)]
pub struct DirDiff {
    entries: Vec<DirDiffEntry>,
}

impl DirDiff {
    // TODO: maybe make this return a Result?
    /// Diffs generated on windows **may not work properly for linux**.
    /// This is due to windows not supporting executable flags needed
    /// on linux, resulting in written binaries without the executable
    /// flag, meaning you'll have to `chmod +x [YOUR BINARY]` manually.
    pub fn new(old_dir: &Path, new_dir: &Path) -> Self {
        let old_dir = &old_dir.canonicalize().unwrap();
        let new_dir = &new_dir.canonicalize().unwrap();

        // ignore .hidden files and files in .gitignore
        let to_walk = ignore::Walk::new(new_dir)
            .map(|x| x.expect("invalid gitignore").into_path())
            .collect::<Vec<_>>();

        let entries: Vec<DirDiffEntry> = to_walk
            .par_iter()
            .filter_map(|path| {
                let canonical_path = path.canonicalize().unwrap();
                // relative to new_dir
                let relative_path = canonical_path.strip_prefix(new_dir).unwrap().to_owned();

                info!("Diffing: {relative_path:?}");

                if path.is_dir() {
                    return Some(DirDiffEntry::Dir(relative_path));
                }

                // skip other stuff like symlinks
                if !path.is_file() {
                    return None;
                }

                let new_file = match fs::read(&canonical_path) {
                    Ok(data) => data,
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => return None,
                    Err(e) => panic!("Error reading file: {:?}", e),
                };
                let old_file = fs::read(old_dir.join(&relative_path)).unwrap_or_default();

                let flags = {
                    #[cfg(target_family = "unix")]
                    {
                        let new_meta = fs::metadata(&canonical_path).unwrap();
                        Some(FileFlags {
                            executable: new_meta.permissions().mode() & 0o111 != 0,
                        })
                    }

                    #[cfg(not(target_family = "unix"))]
                    None
                };

                if rapidhash_v3(&new_file) == rapidhash_v3(&old_file) {
                    return Some(DirDiffEntry::File {
                        path: relative_path,
                        state: DataState::Identical,
                        flags,
                    });
                }

                if old_file.is_empty() {
                    return Some(DirDiffEntry::File {
                        path: relative_path,
                        state: DataState::Raw(new_file.into()),
                        flags,
                    });
                }

                // use a negative compression level to avoid actually
                // compressing the data here, we just want to reference the
                // previous file (we just want to do the delta part here, and
                // compress everything together later, during `.ser()` )
                let level = -5;

                let mut encoder =
                    zstd::Encoder::with_ref_prefix(io::Cursor::new(Vec::new()), level, &old_file)
                        .expect("Couldn't create encoder");

                encoder
                    .write_all(&new_file)
                    .expect("Couldn't write data to encoder");

                let patch = encoder
                    .finish()
                    .expect("Couldn't create patch with encoder")
                    .into_inner();

                Some(DirDiffEntry::File {
                    path: relative_path,
                    state: DataState::Patch(patch.into()),
                    flags,
                })
            })
            .collect();

        Self { entries }
    }

    /// Apply diff in-place to dir, this will overwrite files
    pub fn apply_to(self, dir: &Path, delete_old: bool) -> anyhow::Result<()> {
        let dir = &dir.canonicalize()?;

        let mut unprocessed_entries = self.entries;

        for path in ignore::Walk::new(dir) {
            let path = path.expect("invalid gitignore");

            let canonical_path = path.path().canonicalize().unwrap();
            let relative_path = canonical_path.strip_prefix(dir).unwrap().to_owned();

            if path.path().is_dir() {
                if let Some(i) = unprocessed_entries.iter().position(|entry| match entry {
                    DirDiffEntry::Dir(p) => p == &relative_path,
                    _ => false,
                }) {
                    unprocessed_entries.remove(i);
                } else if delete_old {
                    fs::remove_dir_all(&canonical_path)?;
                    info!("Removed old dir: {relative_path:?}");
                };
            }
            if path.path().is_file() {
                if let Some(i) = unprocessed_entries
                    .iter_mut()
                    .position(|entry| match entry {
                        DirDiffEntry::File { path: p, .. } => p == &relative_path,
                        _ => false,
                    })
                {
                    let DirDiffEntry::File {
                        path: _,
                        state,
                        flags,
                    } = unprocessed_entries.remove(i)
                    else {
                        unreachable!()
                    };
                    match state {
                        DataState::Patch(patch) => {
                            let old_file_data = match fs::read(&canonical_path) {
                                Ok(data) => data,
                                Err(e) if e.kind() == std::io::ErrorKind::NotFound => vec![],
                                Err(e) => Err(e).context("Error reading file to diff")?,
                            };

                            let mut decoder = zstd::Decoder::with_ref_prefix(
                                io::Cursor::new(patch),
                                &old_file_data,
                            )
                            .expect("Couldn't create decoder");

                            let mut new_file_data = Vec::new();
                            decoder
                                .read_to_end(&mut new_file_data)
                                .context("Patcher failed")?;

                            fs::write(&canonical_path, new_file_data)?;
                            info!("Applied diff (patched): {relative_path:?}");
                        }
                        DataState::Raw(data) => {
                            fs::write(&canonical_path, data)?;
                            info!("Applied diff (raw): {relative_path:?}");
                        }
                        DataState::Identical => {
                            info!("Applied diff (identical, unchanged): {relative_path:?}");
                        }
                    }
                    if let Some(x) = flags {
                        #[cfg(target_family = "unix")]
                        {
                            let mut permissions = fs::metadata(&canonical_path)?.permissions();
                            permissions.set_mode(if x.executable {
                                permissions.mode() | 0o111
                            } else {
                                permissions.mode() & !0o111
                            });
                            fs::set_permissions(&canonical_path, permissions)?;
                            info!("Applied file flags: {relative_path:?}");
                        }
                    }
                } else if delete_old {
                    info!("Removed old file: {relative_path:?}");
                    fs::remove_file(&canonical_path)?;
                }
            }
        }

        for entry in unprocessed_entries {
            match entry {
                DirDiffEntry::File {
                    path,
                    state: DataState::Raw(data),
                    flags,
                } => {
                    fs::write(dir.join(&path), data)?;
                    info!("Added new file: {path:?}");
                    if let Some(x) = flags {
                        #[cfg(target_family = "unix")]
                        {
                            let mut permissions = fs::metadata(dir.join(&path))
                                .context("Couldn't query metadata for new file")?
                                .permissions();
                            permissions.set_mode(if x.executable {
                                permissions.mode() | 0o111
                            } else {
                                permissions.mode() & !0o111
                            });
                            fs::set_permissions(&path, permissions)?;
                            info!("Applied file flags: {path:?}");
                        }
                    }
                }
                DirDiffEntry::File {
                    path,
                    state: DataState::Patch(_) | DataState::Identical,
                    ..
                } => {
                    error!("File at path `{path:?}` wasn't found; cannot apply diff. Continuing...")
                }
                DirDiffEntry::Dir(path) => {
                    fs::create_dir_all(dir.join(&path))?;
                    info!("Added new dir: {path:?}");
                }
            }
        }

        Ok(())
    }
}

// BOBDIFF + 1 byte for version
pub const MAGIC_VER: u8 = 3;
pub const MAGIC_BYTES: [u8; 8] = [b'B', b'O', b'B', b'D', b'I', b'F', b'F', MAGIC_VER];

impl DirDiff {
    pub fn ser(&self) -> Vec<u8> {
        let mut ser = Vec::new();
        ser.extend_from_slice(&MAGIC_BYTES);

        let uncompressed_raw = &rkyv::to_bytes::<rancor::Error>(self).unwrap();

        // level 22 here seems to perform better than 19 in terms of both time
        // and size (~3x)
        let mut encoder = zstd::Encoder::new(io::Cursor::new(Vec::new()), 22)
            .expect("Couldn't create final encoder");

        encoder
            .multithread(available_parallelism().map(|x| x.get()).unwrap_or(4) as u32)
            .expect("couldn't enable multithreading");

        encoder
            .write_all(&uncompressed_raw)
            .expect("Couldn't write data to final encoder");

        let compressed_raw = encoder
            .finish()
            .expect("Couldn't compress with final encoder")
            .into_inner();

        ser.extend_from_slice(&compressed_raw);

        ser
    }
    pub fn check_compat(serialized: &[u8]) -> bool {
        serialized[0..MAGIC_BYTES.len()] == MAGIC_BYTES
    }
    pub fn deser(serialized: &[u8]) -> anyhow::Result<Self> {
        if serialized[0..7] != MAGIC_BYTES[0..7] {
            return Err(anyhow!("Invalid magic bytes"));
        }

        if serialized[7] != MAGIC_BYTES[7] {
            return Err(anyhow!("Bobdiff version mismatch, cannot parse"));
        }

        let uncompressed_raw =
            zstd::decode_all(&serialized[8..]).context("zstd decompression failed")?;

        Ok(rkyv::from_bytes::<_, rancor::Error>(&uncompressed_raw)?)
    }
}
