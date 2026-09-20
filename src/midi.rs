use anyhow::{anyhow, Result};
use midir::{MidiOutput, MidiOutputConnection, MidiOutputPort};
use std::{
    collections::VecDeque,
    sync::{mpsc, Arc, Mutex},
    thread,
    time::Instant,
};

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

pub struct OutputQueue {
    tx: mpsc::Sender<(Instant, String, i32, u8, u8)>,
    errors: Arc<Mutex<VecDeque<String>>>,
}
impl OutputQueue {
    pub fn new(output: SharedOutput) -> Self {
        let (tx, rx) = mpsc::channel::<(Instant, String, i32, u8, u8)>();
        let errors = Arc::new(Mutex::new(VecDeque::new()));
        let worker_errors = errors.clone();
        thread::spawn(move || {
            while let Ok((deadline, kind, note, velocity, channel)) = rx.recv() {
                let now = Instant::now();
                if deadline > now {
                    thread::sleep(deadline.duration_since(now));
                }
                if let Err(error) = output.lock().unwrap().send(&kind, note, velocity, channel) {
                    let mut errors = worker_errors.lock().unwrap();
                    if errors.len() >= 32 {
                        errors.pop_front();
                    }
                    errors.push_back(error.to_string());
                }
            }
        });
        Self { tx, errors }
    }
    pub fn send_at(
        &self,
        deadline: Instant,
        kind: String,
        note: i32,
        velocity: u8,
        channel: u8,
    ) -> Result<()> {
        self.tx
            .send((deadline, kind, note, velocity, channel))
            .map_err(|e| anyhow!(e.to_string()))
    }
    pub fn drain_errors(&self) -> Vec<String> {
        self.errors.lock().unwrap().drain(..).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
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
    #[test]
    fn queue_delivers_after_deadline() {
        let backend: SharedOutput = Arc::new(Mutex::new(Box::new(MonitorOutput::default())));
        let q = OutputQueue::new(backend.clone());
        q.send_at(Instant::now(), "on".into(), 60, 100, 1).unwrap();
        thread::sleep(Duration::from_millis(10));
        assert!(backend.lock().unwrap().send("off", 60, 0, 1).is_ok());
    }
}
