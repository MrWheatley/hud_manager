use std::cmp::Ordering;
use std::collections::HashSet;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

use anyhow::{bail, ensure, Context, Result};
use walkdir::WalkDir;

const HUDS: &str = "huds";
const INFO_VDF: &str = "info.vdf";
const FAVORITES_TXT: &str = "favorites.txt";

#[derive(Default)]
pub struct Huds {
    pub huds: Vec<Hud>,
    pub active_hud: Option<Hud>,
    favorites: HashSet<String>,
}

impl Huds {
    pub fn scan_for_huds(&mut self) -> Result<()> {
        let custom_dir = custom_dir()?;

        self.huds.clear();

        let walk_dir = |d| WalkDir::new(d).max_depth(2).into_iter().flatten();
        let is_vdf = |e: &walkdir::DirEntry| e.path().ends_with(INFO_VDF);
        let hud_from_vdf = |e| {
            let mut hud = Hud::from_vdf(e);
            hud.favorite = self.favorites.contains(&hud.name);
            hud
        };

        if let Some(entry) = walk_dir(&custom_dir).find(is_vdf) {
            let hud = hud_from_vdf(entry.into_path());
            self.huds.push(hud.clone());
            self.active_hud = Some(hud);
        }

        for entry in walk_dir(&custom_dir.join(HUDS)).filter(is_vdf) {
            let hud = hud_from_vdf(entry.into_path());
            self.huds.push(hud);
        }

        self.huds.sort_unstable();

        Ok(())
    }

    pub fn set_active_hud(&mut self, hud: &str) -> Result<()> {
        let custom_dir = custom_dir()?;

        if let Some(active_hud) = self.active_hud.as_ref().filter(|h| h.path.exists()) {
            ensure!(hud != active_hud.name, "hud already active");

            let to = custom_dir.join(format!("{HUDS}/{}", active_hud.name));
            fs::rename(&active_hud.path, &to).with_context(|| "failed to move hud")?;

            let hud = find_hud(&mut self.huds, &active_hud.name);
            hud.path = to;
        }

        let hud = find_hud(&mut self.huds, hud);

        let to = custom_dir.join(&hud.name);
        fs::rename(&hud.path, &to).with_context(|| "failed to move hud")?;

        hud.path = to;
        self.active_hud = Some(hud.clone());

        fn find_hud<'a>(huds: &'a mut [Hud], hud: &str) -> &'a mut Hud {
            huds.iter_mut()
                .find(|h| h.name == hud)
                .expect("hud should exist")
        }

        Ok(())
    }

    pub fn save_favorites(&mut self) -> Result<()> {
        let huds_dir = custom_dir()?.join(HUDS);
        let favorites = huds_dir.join(FAVORITES_TXT);

        if !huds_dir.exists() {
            fs::create_dir_all(huds_dir)?;
        }

        let mut file = fs::File::create(favorites)?;
        self.favorites.clear();
        file.write_all(
            self.huds
                .iter()
                .take_while(|h| h.favorite)
                .map(|h| h.name.as_str())
                .inspect(|h| {
                    self.favorites.insert(h.to_string());
                })
                .collect::<Vec<_>>()
                .join("\n")
                .as_bytes(),
        )?;

        Ok(())
    }

    pub fn update_favorites(&mut self) -> Result<()> {
        let huds_dir = custom_dir()?.join(HUDS);
        let favorites = huds_dir.join(FAVORITES_TXT);

        if !favorites.exists() {
            fs::create_dir_all(huds_dir)?;
            fs::File::create(&favorites)?;
            return Ok(());
        }

        let favorites =
            fs::read_to_string(favorites).with_context(|| "failed to read `favorites.txt`")?;

        self.favorites = favorites.lines().map(|l| l.to_string()).collect();

        Ok(())
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct Hud {
    pub name: String,
    pub path: PathBuf,
    pub favorite: bool,
}

impl Hud {
    fn from_vdf(vdf: PathBuf) -> Self {
        let mut path = vdf;
        path.pop();

        let name = path.file_name().unwrap().to_string_lossy().to_string();

        Self {
            name,
            path,
            favorite: false,
        }
    }
}

impl Ord for Hud {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self.favorite, other.favorite) {
            (true, true) => self.name.cmp(&other.name),
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            (false, false) => self.name.cmp(&other.name),
        }
    }
}

impl PartialOrd for Hud {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn custom_dir() -> Result<PathBuf> {
    let mut custom_dir =
        std::env::current_exe().with_context(|| "failed to get current exe dir")?;
    custom_dir.pop();

    if custom_dir.ends_with("custom/huds") {
        custom_dir.pop();
    } else if !custom_dir.ends_with("custom") {
        bail!("exe must be in `custom` or `custom\\huds`");
    }

    Ok(custom_dir)
}
