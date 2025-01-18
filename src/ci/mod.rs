use anyhow::anyhow;
use std::fs;
use std::path::PathBuf;

pub fn command(dir: PathBuf) -> anyhow::Result<()> {
    if !fs::exists(&dir)? {
        return Err(anyhow!("Directory doesn't exist"));
    }

    // #### do all ci steps:
    // (pre-bob step) download last bob_build.tar.xz into bob_build
    // 1. create bob_build_x (where x is every supported platform; x86_64-pc-windows-msvc etc)
    //    by filtering only files for platform x
    // 2. move all of these into a "old" folder (or something similar)
    // 3. copy ./old/bob_build ./bob_build
    // 4. run bob build
    // 5. filter out to platform specific folders again (like step 1)
    // 6. move dirs named bob_build* to ./new folder
    // 7. diff ./old/* with ./new/*
    // 8. compress ./new/* (resulting in bob_build.tar.xz + bob_build_x.tar.xz * platforms)
    // 9. compress ./diff/* (resulting in bob_build.diff.xz + bob_build_x.diff.xz * platforms)
    // 10. move all .xz files into final directory, these files should be uploaded to github releases

    Ok(())
}
