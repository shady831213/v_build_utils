use std::env;
use std::fs;
use std::path::{Path, PathBuf};

pub fn walk_dir<P: AsRef<Path> + Copy, F: FnMut(&PathBuf) -> Result<(), String>>(
    dir: P,
    f: &mut F,
) -> Result<(), String> {
    for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
        let path = entry.map_err(|e| e.to_string())?.path();
        f(&path)?;
        if path.is_dir() {
            walk_dir(&path, f)?
        }
    }
    Ok(())
}

pub fn copy_dir<P1: AsRef<Path> + Copy, P2: AsRef<Path> + Copy>(
    src: P1,
    dest: P2,
) -> Result<(), String> {
    fs::create_dir_all(dest).map_err(|e| format!("{:?}:{:?}", dest.as_ref().display(), e))?;
    walk_dir(src, &mut |p: &PathBuf| {
        let dest_p = dest.as_ref().join(p.strip_prefix(src).unwrap());
        println!("cargo:rerun-if-changed={}", p.display());
        if p.is_dir() {
            fs::create_dir_all(&dest_p).map_err(|e| format!("{:?}:{:?}", dest_p.display(), e))?;
        } else {
            fs::copy(&p, &dest_p)
                .map_err(|e| format!("Copy {:?} to {:?}:{:?}", p.display(), dest_p.display(), e))?;
        }
        Ok(())
    })
}

pub fn link_dir<P1: AsRef<Path> + Copy, P2: AsRef<Path> + Copy>(
    src: P1,
    dest: P2,
) -> Result<(), String> {
    fs::create_dir_all(dest).map_err(|e| format!("{:?}:{:?}", dest.as_ref().display(), e))?;
    walk_dir(src, &mut |p: &PathBuf| {
        let dest_p = dest.as_ref().join(p.strip_prefix(&src).unwrap());
        println!("cargo:rerun-if-changed={}", p.display());
        if p.is_dir() {
            fs::create_dir_all(&dest_p).map_err(|e| format!("{:?}:{:?}", dest_p.display(), e))?;
        } else {
            use std::os::unix::fs::symlink;
            if dest_p.exists() {
                fs::remove_file(&dest_p).map_err(|e| {
                    format!("Link {:?} to {:?}:{:?}", p.display(), dest_p.display(), e)
                })?;
            }
            symlink(&p, &dest_p)
                .map_err(|e| format!("Link {:?} to {:?}:{:?}", p.display(), dest_p.display(), e))?;
        }
        Ok(())
    })
}

pub fn dep_value(dep: &str, key: &str) -> Result<String, String> {
    let env_name = ["DEP", &dep.to_uppercase(), &key.to_uppercase()].join("_");
    let dir = env::var(&env_name).map_err(|e| e.to_string())?;
    Ok(dir)
}

pub struct OtherDir {
    key: String,
    root: PathBuf,
}

impl OtherDir {
    pub fn new(key: &str) -> Result<Self, env::VarError> {
        let out_dir = PathBuf::from(env::var("OUT_DIR")?);
        let header_root = out_dir.join(env::var("CARGO_MANIFEST_LINKS")?);
        println!("cargo:{}={}", key, header_root.display());
        Ok(OtherDir {
            key: key.to_string(),
            root: header_root,
        })
    }
    pub fn add_dir<P: AsRef<Path> + Copy>(&self, dir: P) -> Result<&Self, String> {
        copy_dir(dir, &self.root)?;
        Ok(self)
    }
    pub fn add_dep(&self, dep: &str) -> Result<&Self, String> {
        let dir = dep_value(dep, &self.key)?;
        copy_dir(&dir, &self.root.join(dep))?;
        Ok(self)
    }
    pub fn merge_dep(&self, dep: &str) -> Result<&Self, String> {
        let dir = dep_value(dep, &self.key)?;
        copy_dir(&dir, &self.root)?;
        Ok(self)
    }
}

pub fn target_dir() -> PathBuf {
    let root = if let Ok(v) = env::var("CARGO_TARGET_DIR") {
        PathBuf::from(v)
    } else {
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("target")
    };
    let target = env::var("TARGET").unwrap();
    let host = env::var("HOST").unwrap();
    let profile = env::var("PROFILE").unwrap();
    if target == host {
        root.join(&profile)
    } else {
        root.join(&target).join(&profile)
    }
}
