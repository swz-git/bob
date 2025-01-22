// diffing cli tool + built-in library that wraps qbsdiff
// we might be able to use something that already exists, but it'll need to support diffing of folders

use anyhow::{anyhow, Context};
use log::info;
use qbsdiff::{Bsdiff, Bspatch};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::fs;
use std::io::{stdin, stdout, BufReader, Cursor, Read, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq)]
enum DirDiffEntry {
    File {
        path: PathBuf,
        /// Some(Vec<u8> of qbsdiff patch) if file
        /// None if dir
        patch: Vec<u8>,
    },
    Dir(PathBuf),
}

pub struct DirDiff {
    entries: Vec<DirDiffEntry>,
}

impl DirDiff {
    // TODO: maybe make this return a Result?
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

                let new_file = match fs::read(canonical_path) {
                    Ok(data) => data,
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => return None,
                    Err(e) => panic!("Error reading file: {:?}", e),
                };
                let old_file = fs::read(old_dir.join(&relative_path)).unwrap_or_default();

                let mut patch = Vec::new();
                Bsdiff::new(&old_file, &new_file)
                    .compare(&mut patch)
                    .expect("generating diff failed");

                Some(DirDiffEntry::File {
                    path: relative_path,
                    patch,
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
                } else {
                    if delete_old {
                        fs::remove_dir_all(&canonical_path)?;
                        info!("Removed old dir: {relative_path:?}");
                    }
                };
            }
            if path.path().is_file() {
                if let Some(i) = unprocessed_entries.iter().position(|entry| match entry {
                    DirDiffEntry::File { path: p, .. } => p == &relative_path,
                    _ => false,
                }) {
                    let entry = unprocessed_entries.remove(i);
                    if let DirDiffEntry::File { patch, .. } = entry {
                        let old_file_data = match fs::read(&canonical_path) {
                            Ok(data) => data,
                            Err(e) if e.kind() == std::io::ErrorKind::NotFound => vec![],
                            Err(e) => Err(e).context("Error reading file to for diff")?,
                        };

                        let patcher = Bspatch::new(&patch)?;
                        let mut new_file_data =
                            Vec::with_capacity(patcher.hint_target_size() as usize);
                        patcher.apply(&old_file_data, Cursor::new(&mut new_file_data))?;

                        info!("Applied diff: {relative_path:?}");
                        fs::write(&canonical_path, new_file_data)?;
                    }
                } else {
                    if delete_old {
                        info!("Removed old file: {relative_path:?}");
                        fs::remove_file(&canonical_path)?;
                    }
                }
            }
        }

        for entry in unprocessed_entries {
            match entry {
                DirDiffEntry::File { path, patch } => {
                    let patcher = Bspatch::new(&patch)?;
                    let mut new_file_data = Vec::with_capacity(patcher.hint_target_size() as usize);
                    patcher.apply(&[], Cursor::new(&mut new_file_data))?;
                    fs::write(dir.join(&path), new_file_data)?;
                    info!("Added new file: {path:?}");
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
const MAGIC_BYTES: [u8; 8] = [b'B', b'O', b'B', b'D', b'I', b'F', b'F', 0];

impl DirDiff {
    pub fn ser(&self) -> Vec<u8> {
        let mut ser = Vec::new();
        ser.extend_from_slice(&MAGIC_BYTES);
        for entry in &self.entries {
            match entry {
                DirDiffEntry::File { path, patch } => {
                    ser.extend_from_slice(b"F");

                    let path_data = path.to_string_lossy();
                    ser.extend_from_slice(&(path_data.len() as u32).to_be_bytes());
                    ser.extend_from_slice(path_data.as_bytes());

                    ser.extend_from_slice(&(patch.len() as u32).to_be_bytes());
                    ser.extend_from_slice(&patch);
                }
                DirDiffEntry::Dir(path) => {
                    ser.extend_from_slice(b"D");

                    let path_data = path.to_string_lossy();
                    ser.extend_from_slice(&(path_data.len() as u32).to_be_bytes());
                    ser.extend_from_slice(path_data.as_bytes());
                }
            }
        }
        ser
    }
    pub fn deser(serialized: &[u8]) -> anyhow::Result<Self> {
        if serialized[0..8] != MAGIC_BYTES {
            return Err(anyhow!("Invalid magic bytes"));
        }

        let mut file_diffs = vec![];

        let mut payload_reader = BufReader::new(&serialized[8..]);

        // start at 1 because version is a single byte
        let mut data_type_buf = [0u8];
        while let Ok(_) = payload_reader.read_exact(&mut data_type_buf) {
            let entry_type = data_type_buf[0];

            match entry_type {
                b'F' => {
                    let mut path_len_buf = [0u8; 4];
                    payload_reader.read_exact(&mut path_len_buf)?;
                    let path_len = u32::from_be_bytes(path_len_buf) as usize;

                    let mut path_buf = vec![0u8; path_len];
                    payload_reader.read_exact(&mut path_buf)?;
                    let path = PathBuf::from(String::from_utf8(path_buf)?);

                    let mut patch_len_buf = [0u8; 4];
                    payload_reader.read_exact(&mut patch_len_buf)?;
                    let patch_len = u32::from_be_bytes(patch_len_buf) as usize;

                    let mut patch = vec![0u8; patch_len];
                    payload_reader.read_exact(&mut patch)?;

                    file_diffs.push(DirDiffEntry::File { path, patch });
                }
                b'D' => {
                    let mut path_len_buf = [0u8; 4];
                    payload_reader.read_exact(&mut path_len_buf)?;
                    let path_len = u32::from_be_bytes(path_len_buf) as usize;

                    let mut path_buf = vec![0u8; path_len];
                    payload_reader.read_exact(&mut path_buf)?;
                    let path = PathBuf::from(String::from_utf8(path_buf)?);

                    file_diffs.push(DirDiffEntry::Dir(path));
                }
                _ => return Err(anyhow!("Invalid entry type")),
            }
        }

        Ok(Self {
            entries: file_diffs,
        })
    }
}

pub fn command_diff(old: PathBuf, new: PathBuf) -> anyhow::Result<()> {
    if !fs::exists(&old)? {
        return Err(anyhow!("Directory doesn't exist: {old:?}"));
    }
    if !fs::exists(&new)? {
        return Err(anyhow!("Directory doesn't exist: {new:?}"));
    }

    // use the DirDiff struct to diff the directory
    let diff = DirDiff::new(&old, &new);

    stdout().write(&diff.ser())?;

    Ok(())
}

pub fn command_diff_apply(dir: PathBuf) -> anyhow::Result<()> {
    if !fs::exists(&dir)? {
        return Err(anyhow!("Directory doesn't exist"));
    }

    let mut serialized = Vec::new();
    stdin().read_to_end(&mut serialized)?;
    let diff = DirDiff::deser(&serialized)?;

    diff.apply_to(&dir, true)?;

    Ok(())
}
