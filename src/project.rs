use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

pub const STEP_COUNT: usize = 16;
pub const DURATIONS: [f32; 4] = [0.125, 0.25, 0.5, 1.0];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StepFile {
    pub note: Option<u8>,
    pub velocity: u8,
    pub duration: f32,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct PatternFile {
    steps: Vec<StepFile>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct ProjectFile {
    name: String,
    tempo: f32,
    pattern: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Project {
    pub dir: PathBuf,
    pub name: String,
    pub tempo: f32,
    pub steps: Vec<StepFile>,
    pub pattern_path: String,
}

pub fn note_name(note: Option<u8>) -> String {
    let Some(note) = note else {
        return "REST".into();
    };
    const NAMES: [&str; 12] = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    format!("{}{}", NAMES[(note % 12) as usize], (note / 12) as i16 - 1)
}
pub fn validate_steps(steps: &[StepFile]) -> Result<()> {
    if steps.len() != STEP_COUNT {
        return Err(anyhow!("a demo pattern must contain exactly 16 steps"));
    }
    for step in steps {
        if step.velocity > 127 {
            return Err(anyhow!("velocity must be 0..127"));
        }
        if !DURATIONS
            .iter()
            .any(|x| (x - step.duration).abs() < f32::EPSILON)
        {
            return Err(anyhow!("duration is not a supported musical value"));
        }
    }
    Ok(())
}
pub fn validate_tempo(tempo: f32) -> Result<()> {
    if (30.0..=300.0).contains(&tempo) {
        Ok(())
    } else {
        Err(anyhow!("tempo must be between 30 and 300 BPM"))
    }
}
pub fn example_project(dir: impl Into<PathBuf>) -> Project {
    let notes = [
        60, 60, 63, 65, 63, 60, 58, 55, 60, 67, 65, 63, 60, 58, 55, 53,
    ];
    let velocities = [
        110, 88, 104, 96, 90, 108, 92, 82, 105, 94, 112, 98, 86, 100, 90, 78,
    ];
    Project {
        dir: dir.into(),
        name: "first-light".into(),
        tempo: 120.0,
        pattern_path: "patterns/bass.toml".into(),
        steps: notes
            .into_iter()
            .zip(velocities)
            .map(|(note, velocity)| StepFile {
                note: Some(note),
                velocity,
                duration: 0.25,
            })
            .collect(),
    }
}
pub fn load(dir: &Path) -> Result<Project> {
    let meta: ProjectFile = toml::from_str(&fs::read_to_string(dir.join("project.toml"))?)
        .context("invalid project.toml")?;
    validate_tempo(meta.tempo)?;
    let pattern_path = dir.join(&meta.pattern);
    let pattern: PatternFile = toml::from_str(&fs::read_to_string(&pattern_path)?)
        .with_context(|| format!("invalid {}", pattern_path.display()))?;
    validate_steps(&pattern.steps)?;
    Ok(Project {
        dir: dir.to_path_buf(),
        name: meta.name,
        tempo: meta.tempo,
        steps: pattern.steps,
        pattern_path: meta.pattern,
    })
}
fn atomic_write(path: &Path, data: &[u8]) -> Result<()> {
    let tmp = path.with_extension("index-tmp");
    let mut file = fs::File::create(&tmp)?;
    file.write_all(data)?;
    file.sync_all()?;
    drop(file);
    fs::rename(tmp, path)?;
    Ok(())
}
pub fn save(project: &Project) -> Result<()> {
    validate_tempo(project.tempo)?;
    validate_steps(&project.steps)?;
    fs::create_dir_all(project.dir.join("patterns"))?;
    let meta = ProjectFile {
        name: project.name.clone(),
        tempo: project.tempo,
        pattern: project.pattern_path.clone(),
    };
    atomic_write(
        &project.dir.join("project.toml"),
        toml::to_string_pretty(&meta)?.as_bytes(),
    )?;
    atomic_write(
        &project.dir.join(&project.pattern_path),
        toml::to_string_pretty(&PatternFile {
            steps: project.steps.clone(),
        })?
        .as_bytes(),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn has_sixteen_steps() {
        assert_eq!(example_project(".").steps.len(), 16);
    }
    #[test]
    fn note_names_include_rest() {
        assert_eq!(note_name(Some(60)), "C4");
        assert_eq!(note_name(Some(63)), "D#4");
        assert_eq!(note_name(None), "REST");
    }
    #[test]
    fn bounds_are_checked() {
        assert!(validate_steps(&example_project(".").steps).is_ok());
        assert!(validate_tempo(29.0).is_err());
    }
    #[test]
    fn round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let p = example_project(dir.path());
        save(&p).unwrap();
        assert_eq!(load(dir.path()).unwrap(), p);
    }
}
