use anyhow::{anyhow, Result};
use midir::{MidiOutput, MidiOutputConnection, MidiOutputPort};
use std::sync::{Arc, Mutex};

pub fn note_bytes(kind: &str, note: i32, velocity: u8, channel: u8) -> Result<[u8; 3]> {
    if !(1..=16).contains(&channel) || !(0..=127).contains(&note) || velocity > 127 {
        return Err(anyhow!("invalid MIDI event"));
    }
    let status = if kind == "on" {
        0x90
    } else if kind == "off" {
        0x80
    } else {
        return Err(anyhow!("unknown MIDI event kind"));
    };
    Ok([status | (channel - 1), note as u8, velocity])
}

pub trait OutputBackend: Send {
    fn send(&mut self, kind: &str, note: i32, velocity: u8, channel: u8) -> Result<()>;
}
#[derive(Default)]
pub struct MonitorOutput {
    pub events: Vec<[u8; 3]>,
}
impl OutputBackend for MonitorOutput {
    fn send(&mut self, kind: &str, note: i32, velocity: u8, channel: u8) -> Result<()> {
        self.events.push(note_bytes(kind, note, velocity, channel)?);
        Ok(())
    }
}
pub struct MidiOutputBackend {
    connection: MidiOutputConnection,
}
impl MidiOutputBackend {
    pub fn ports() -> Result<Vec<(String, MidiOutputPort)>> {
        let out = MidiOutput::new("Index")?;
        Ok(out
            .ports()
            .into_iter()
            .map(|p| {
                (
                    out.port_name(&p)
                        .unwrap_or_else(|_| "Unknown MIDI port".into()),
                    p,
                )
            })
            .collect())
    }
    pub fn open(port: &MidiOutputPort, _name: String) -> Result<Self> {
        let out = MidiOutput::new("Index")?;
        Ok(Self {
            connection: out
                .connect(port, "Index output")
                .map_err(|e| anyhow!(e.to_string()))?,
        })
    }
}
impl OutputBackend for MidiOutputBackend {
    fn send(&mut self, kind: &str, note: i32, velocity: u8, channel: u8) -> Result<()> {
        self.connection
            .send(&note_bytes(kind, note, velocity, channel)?)
            .map_err(|e| anyhow!(e.to_string()))
    }
}
pub type SharedOutput = Arc<Mutex<Box<dyn OutputBackend>>>;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn channel_is_zero_based() {
        assert_eq!(note_bytes("on", 60, 100, 1).unwrap(), [0x90, 60, 100]);
        assert_eq!(note_bytes("off", 60, 0, 16).unwrap(), [0x8f, 60, 0]);
    }
    #[test]
    fn monitor_delivers() {
        let mut m = MonitorOutput::default();
        m.send("on", 60, 100, 2).unwrap();
        assert_eq!(m.events[0], [0x91, 60, 100]);
    }
}
