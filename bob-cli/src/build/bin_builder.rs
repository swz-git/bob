use anyhow::{Context, anyhow};
use bob_lib::dirhasher;
use log::info;
use std::io::Write;
use std::path::Path;
use std::{env, fs, path::PathBuf, process};

use crate::config::{BobConfig, BuilderConfigVariant};

fn generate_dockerfile(
    variant: &BuilderConfigVariant,
    project_root: &Path,
) -> anyhow::Result<String> {
    let mut tt = tinytemplate::TinyTemplate::new();
    let generic = variant.get_inner_as_generic();
    let contents = generic.get_dockerfile_contents(project_root)?;
    tt.add_template("x", &contents)
        .context("Dockerfile was not a valid tinytemplate")?;
    match variant {
        BuilderConfigVariant::Custom(custom) => tt.render("x", &custom.values).map_err(Into::into),
        _ => tt.render("x", &generic).map_err(Into::into),
    }
}

pub struct BuildResult {
    pub tar_binary: Vec<u8>,
    pub dir_hash: u64,
}

mod uid {
    use std::sync::atomic::{AtomicUsize, Ordering};

    pub static COUNTER: AtomicUsize = AtomicUsize::new(0);
    pub(super) fn uid() -> Box<str> {
        format!(
            "{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        )
        .into()
    }
}
use uid::uid;

// Returns Ok(None) if hash matches
pub fn build(
    project_root: PathBuf,
    build_config: &BobConfig,
    prev_hash: Option<u64>,
) -> anyhow::Result<Option<BuildResult>> {
    let project_root = project_root.canonicalize()?;

    let dockerfile_content = generate_dockerfile(&build_config.builder_config, &project_root)
        .context("Generating dockerfile")?;
    let tempfile_path = env::temp_dir().join(format!("Dockerfile-{}", uid()));

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
