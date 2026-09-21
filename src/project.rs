use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Pattern {
    pub name: String,
    pub degrees: Vec<i32>,
    pub durations: Vec<f32>,
    pub velocities: Vec<u8>,
    #[serde(default = "default_gate")]
    pub gate: f32,
    pub channel: u8,
    pub destination: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjectMeta {
    pub name: String,
    pub tempo: f32,
    #[serde(default = "default_pattern_ref")]
    pub pattern: String,
    #[serde(default)]
    pub device: DeviceConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeviceConfig {
    #[serde(default = "default_backend")]
    pub backend: String,
    #[serde(default = "default_destination")]
    pub logical_name: String,
    #[serde(default)]
    pub system_port: Option<String>,
}
fn default_backend() -> String {
    "monitor".into()
}
fn default_destination() -> String {
    "Monitor".into()
}
impl Default for DeviceConfig {
    fn default() -> Self {
        Self {
            backend: default_backend(),
            logical_name: default_destination(),
            system_port: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Project {
    pub dir: PathBuf,
    pub meta: ProjectMeta,
    pub pattern: Pattern,
}

fn default_gate() -> f32 {
    0.8
}
fn default_pattern_ref() -> String {
    "patterns/bass.toml".into()
}

pub fn example_project(dir: impl Into<PathBuf>) -> Project {
    Project {
        dir: dir.into(),
        meta: ProjectMeta {
            name: "first-light".into(),
            tempo: 120.0,
            pattern: "patterns/bass.toml".into(),
            device: DeviceConfig::default(),
        },
        pattern: Pattern {
            name: "bass".into(),
            degrees: vec![0, 0, 3, 5, 3, 0, -2, -5, 0, 7, 5, 3, 0, -2, -5, -7],
            durations: vec![0.25; 16],
            velocities: vec![
                105, 90, 110, 100, 90, 105, 95, 85, 100, 92, 108, 98, 88, 102, 94, 82,
            ],
            gate: 0.8,
            channel: 1,
            destination: "Monitor".into(),
        },
    }
}

pub fn validate(p: &Pattern) -> Result<()> {
    if p.name.trim().is_empty()
        || p.degrees.is_empty()
        || p.degrees.len() != p.durations.len()
        || p.degrees.len() != p.velocities.len()
    {
        return Err(anyhow!(
            "pattern requires equally-sized non-empty degrees, durations and velocities"
        ));
    }
    if p.durations.iter().any(|d| !d.is_finite() || *d <= 0.0) {
        return Err(anyhow!("durations must be finite and positive"));
    }
    if !p.gate.is_finite() || !(0.0..=1.0).contains(&p.gate) {
        return Err(anyhow!("gate must be between 0 and 1"));
    }
    if p.velocities.iter().any(|v| *v > 127)
        || !(1..=16).contains(&p.channel)
        || p.destination.trim().is_empty()
    {
        return Err(anyhow!("MIDI velocity/channel/destination out of range"));
    }
    Ok(())
}

pub fn load(dir: &Path) -> Result<Project> {
    let meta: ProjectMeta = toml::from_str(&fs::read_to_string(dir.join("project.toml"))?)
        .context("invalid project.toml")?;
    let pattern_path = dir.join(&meta.pattern);
    let pattern: Pattern = toml::from_str(&fs::read_to_string(&pattern_path)?)
        .with_context(|| format!("invalid {}", pattern_path.display()))?;
    validate(&pattern)?;
    Ok(Project {
        dir: dir.to_path_buf(),
        meta,
        pattern,
    })
}

fn atomic_write(path: &Path, data: &[u8]) -> Result<()> {
    let tmp = path.with_extension(format!(
        "{}tmp",
        path.extension()
            .and_then(|x| x.to_str())
            .map(|x| format!("{x}."))
            .unwrap_or_default()
    ));
    let mut file = fs::File::create(&tmp)?;
    file.write_all(data)?;
    file.sync_all()?;
    drop(file);
    fs::rename(tmp, path)?;
    Ok(())
}

pub fn save(p: &Project) -> Result<()> {
    validate(&p.pattern)?;
    fs::create_dir_all(p.dir.join("patterns"))?;
    atomic_write(
        &p.dir.join("project.toml"),
        toml::to_string_pretty(&p.meta)?.as_bytes(),
    )?;
    atomic_write(
        &p.dir.join(&p.meta.pattern),
        toml::to_string_pretty(&p.pattern)?.as_bytes(),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn separate_authority_roundtrip() {
        let d = tempfile::tempdir().unwrap();
        let p = example_project(d.path());
        save(&p).unwrap();
        let q = load(d.path()).unwrap();
        assert_eq!(p, q);
        assert!(toml::to_string(&q.meta)
            .unwrap()
            .contains("patterns/bass.toml"));
    }
    #[test]
    fn invalid_rejected() {
        let mut p = example_project(".").pattern;
        p.durations.pop();
        assert!(validate(&p).is_err());
    }
    #[test]
    fn atomic_save_leaves_no_tmp() {
        let d = tempfile::tempdir().unwrap();
        let p = example_project(d.path());
        save(&p).unwrap();
        assert!(!d.path().join("project.toml.tmp").exists());
    }
}
