use std::fs;

type DynError = Box<dyn std::error::Error>;

fn main() -> Result<(), DynError> {
    match std::env::args().nth(1).as_deref() {
        Some("gen-test-huds") => gen_test_huds()?,
        _ => print_help(),
    }

    Ok(())
}

fn gen_test_huds() -> Result<(), DynError> {
    let huds_dir = std::path::Path::new(&env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(1)
        .unwrap()
        .to_path_buf()
        .join("target/debug/custom/huds");

    if !huds_dir.exists() {
        fs::create_dir_all(&huds_dir)?;
    }

    let mut name_gen = names::Generator::default();

    for _ in 0..50 {
        let hud = huds_dir.join(name_gen.next().unwrap() + "-hud");

        fs::create_dir_all(&hud)?;
        fs::File::create(hud.join("info.vdf"))?;
    }

    Ok(())
}

fn print_help() {
    eprintln!(
        "Tasks:

gen-test-huds        generates a `custom` folder and some hud folders for testing
                     in `target/debug`
"
    )
}
