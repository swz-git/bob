use anyhow::{anyhow, Context};
use log::info;
use nanoid::nanoid;
use rapidhash::RapidInlineHasher;
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::{
    env, fs,
    path::PathBuf,
    process,
};

use crate::config::{BuildConfig, BuilderConfig};

fn generate_dockerfile(build_config: &BuilderConfig) -> anyhow::Result<String> {
    let mut tt = tinytemplate::TinyTemplate::new();
    Ok(match build_config {
        BuilderConfig::PyInstaller(bc) => {
            tt.add_template(
                "x",
                include_str!("../../dockerfiles/pyinstaller.Dockerfile"),
            )?;
            tt.render("x", bc)?
        }
        BuilderConfig::Rust(bc) => {
            tt.add_template("x", include_str!("../../dockerfiles/rust.Dockerfile"))?;
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
    pub tar_binary: Vec<u8>,
    pub dir_hash: u64,
}

// Returns Ok(None) if hash matches
pub fn build(
    project_root: PathBuf,
    build_config: &BuildConfig,
    prev_hash: Option<u64>,
) -> anyhow::Result<Option<BuildResult>> {
    let project_root = project_root.canonicalize()?;

    let dockerfile_content =
        generate_dockerfile(&build_config.builder_config).context("generating dockerfile")?;
    let tempfile_path = env::temp_dir().join(format!("Dockerfile-{}", nanoid!()));

    let mut tempfile = fs::File::create_new(&tempfile_path)?;
    tempfile.write_all(dockerfile_content.as_bytes())?;
    drop(tempfile);

    let hash = dirhasher(project_root.clone())?;

    info!("{project_root:?} - hash: {hash:X}");

    if Some(hash) == prev_hash {
        info!("Old hash matched, wont rebuild");
        return Ok(None);
    }

    info!("No hash match, building");

    let docker_tag = format!("bob_build:{:x}", hash);

    let build_status_code = process::Command::new("docker")
        .args([
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

    if !build_status_code.success() {
        return Err(anyhow!(
            "Docker build failed for bob project {:?}",
            build_config.project_name
        ));
    }

    let bin = process::Command::new("docker")
        .args(["run", "--rm", &docker_tag])
        .stderr(process::Stdio::inherit())
        .current_dir(&project_root)
        .output()?
        .stdout;

    // let docker_rm_status_code = process::Command::new("docker")
    //     .args(&["image", "rm", &docker_tag])
    //     .stdout(process::Stdio::inherit())
    //     .stderr(process::Stdio::inherit())
    //     .status()?;

    // if !docker_rm_status_code.success() {
    //     warn!("Couldn't remove docker image {:?}", docker_tag)
    // }

    fs::remove_file(tempfile_path).context("removing tmp dockerfile")?;

    Ok(Some(BuildResult {
        tar_binary: bin,
        dir_hash: hash,
    }))

    // todo!()
}
