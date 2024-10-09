use anyhow::{anyhow, Context};
use nanoid::nanoid;
use rapidhash::RapidInlineHasher;
use std::hash::{BuildHasher, Hash, Hasher};
use std::io::Write;
use std::{
    env, fs,
    path::{Path, PathBuf},
    process,
};

use crate::config::BuildConfig;

fn generate_dockerfile(build_config: &BuildConfig) -> anyhow::Result<String> {
    let mut tt = tinytemplate::TinyTemplate::new();
    Ok(match build_config {
        BuildConfig::Nuitka(bc) => todo!(),
        BuildConfig::Rust(bc) => {
            tt.add_template("x", include_str!("../dockerfiles/rust.Dockerfile"))?;
            tt.render("x", bc)?
        }
    })
}

fn dirhasher(dir: PathBuf) -> anyhow::Result<u64> {
    let dir = dir.canonicalize()?;

    let mut paths = vec![];

    for result in ignore::WalkBuilder::new(&dir)
        .hidden(true)
        .git_ignore(true)
        .build()
    {
        let path = result?.into_path();
        if path.is_file() {
            paths.push(path)
        }
    }

    paths.sort();

    let mut hasher = RapidInlineHasher::default();

    for path in paths {
        let content = fs::read(&path).context("hasher couldn't read file")?;
        path.canonicalize()?.strip_prefix(&dir)?.hash(&mut hasher);
        content.hash(&mut hasher);
    }

    Ok(hasher.finish())
}

pub struct BuildResult {
    pub binary: Vec<u8>,
    pub dir_hash: u64,
}

// Returns Ok(None) if hash matches
pub fn build(
    toml_path: PathBuf,
    build_config: BuildConfig,
    prev_hash: Option<u64>,
) -> anyhow::Result<Option<BuildResult>> {
    let project_root = toml_path
        .parent()
        .ok_or(anyhow!("couldn't get parent dir of bob config"))?
        .canonicalize()?;

    let dockerfile_content = generate_dockerfile(&build_config).context("generating dockerfile")?;
    let tempfile_path = env::temp_dir().join(format!("Dockerfile-{}", nanoid!()));

    let mut tempfile = fs::File::create_new(&tempfile_path)?;
    tempfile.write_all(dockerfile_content.as_bytes())?;
    drop(tempfile);

    let hash = dirhasher(project_root.clone())?;

    println!("{project_root:?} - hash: {hash:X}");

    if Some(hash) == prev_hash {
        return Ok(None);
    }

    let docker_tag = format!("bob_build:{:x}", hash);

    process::Command::new("docker")
        .args(&[
            "build",
            "-f",
            tempfile_path.to_str().unwrap(),
            "-t",
            &docker_tag,
            ".",
        ])
        .stdout(process::Stdio::inherit())
        .stderr(process::Stdio::inherit())
        .current_dir(&project_root)
        .status()?;

    let bin = process::Command::new("docker")
        .args(&["run", "--rm", &docker_tag])
        .stderr(process::Stdio::inherit())
        .current_dir(&project_root)
        .output()?
        .stdout;

    Ok(Some(BuildResult {
        binary: bin,
        dir_hash: hash,
    }))

    // todo!()
}
