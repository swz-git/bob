use std::{
    env, fs,
    path::{Path, PathBuf},
    sync::LazyLock,
};

use divan::Bencher;
use humansize::BINARY;
use xz2::read::XzDecoder;

const R23_PATH: LazyLock<PathBuf> =
    LazyLock::new(|| Path::new(&env::temp_dir()).join("BOB_BENCH_R23"));
const R24_PATH: LazyLock<PathBuf> =
    LazyLock::new(|| Path::new(&env::temp_dir()).join("BOB_BENCH_R24"));
const R25_PATH: LazyLock<PathBuf> =
    LazyLock::new(|| Path::new(&env::temp_dir()).join("BOB_BENCH_R25"));

fn download_and_extract(url: &str, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut body = ureq::get(url).call()?.into_body();
    let decompressor = XzDecoder::new(body.as_reader());
    let mut archive = tar::Archive::new(decompressor);
    archive.unpack(path)?;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    download_and_extract(
        "https://github.com/VirxEC/botpack-test/releases/download/r-23/botpack_x86_64-linux.tar.xz",
        &R23_PATH,
    )?;
    download_and_extract(
        "https://github.com/VirxEC/botpack-test/releases/download/r-24/botpack_x86_64-linux.tar.xz",
        &R24_PATH,
    )?;
    download_and_extract(
        "https://github.com/VirxEC/botpack-test/releases/download/r-25/botpack_x86_64-linux.tar.xz",
        &R25_PATH,
    )?;

    // Run registered benchmarks.
    divan::main();

    println!(
        "Patch size r23-r25: {}",
        humansize::format_size(unsafe { PATCH_SIZE_R23_R25 }, BINARY)
    );
    println!(
        "Patch size r24-r25: {}",
        humansize::format_size(unsafe { PATCH_SIZE_R24_R25 }, BINARY)
    );

    fs::remove_dir_all(&*R23_PATH)?;
    fs::remove_dir_all(&*R24_PATH)?;
    fs::remove_dir_all(&*R25_PATH)?;

    Ok(())
}

static mut PATCH_SIZE_R23_R25: usize = 0;
// Time to generate patch r23-r25
#[divan::bench(sample_count = 1)]
fn patch_time_r23_r25(bencher: Bencher) {
    bencher.bench(|| {
        let size = bob_lib::bobdiff::DirDiff::new(&R23_PATH, &R25_PATH)
            .ser()
            .len();
        // this is fine :)
        unsafe { PATCH_SIZE_R23_R25 = size };
    });
}

static mut PATCH_SIZE_R24_R25: usize = 0;
// Time to generate patch r24-r25
#[divan::bench(sample_count = 1)]
fn patch_time_r24_r25(bencher: Bencher) {
    bencher.bench(|| {
        let size = bob_lib::bobdiff::DirDiff::new(&R24_PATH, &R25_PATH)
            .ser()
            .len();
        // this is fine :)
        unsafe { PATCH_SIZE_R24_R25 = size };
    });
}
