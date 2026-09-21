use anyhow::{anyhow, Result};
use midir::{MidiOutput, MidiOutputConnection, MidiOutputPort};
use std::{
    cmp::Ordering,
    collections::BinaryHeap,
    collections::HashMap,
    collections::VecDeque,
    sync::{
        atomic::{AtomicU64, Ordering as AtomicOrdering},
        mpsc, Arc, Mutex,
    },
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
    fn control_change(&mut self, controller: u8, value: u8, channel: u8) -> Result<()>;
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
    fn control_change(&mut self, controller: u8, value: u8, channel: u8) -> Result<()> {
        if !(1..=16).contains(&channel) || controller > 127 || value > 127 {
            return Err(anyhow!("invalid MIDI CC"));
        }
        self.events.push([0xB0 | (channel - 1), controller, value]);
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
    fn control_change(&mut self, controller: u8, value: u8, channel: u8) -> Result<()> {
        if !(1..=16).contains(&channel) || controller > 127 || value > 127 {
            return Err(anyhow!("invalid MIDI CC"));
        }
        self.connection
            .send(&[0xB0 | (channel - 1), controller, value])
            .map_err(|e| anyhow!(e.to_string()))
    }
}
pub type SharedOutput = Arc<Mutex<Box<dyn OutputBackend>>>;

pub struct OutputQueue {
    tx: mpsc::Sender<Command>,
    errors: Arc<Mutex<VecDeque<String>>>,
    generation: Arc<AtomicU64>,
    worker: Mutex<Option<thread::JoinHandle<()>>>,
}
#[derive(Debug)]
struct Event {
    deadline: Instant,
    sequence: u64,
    kind: String,
    note: i32,
    velocity: u8,
    channel: u8,
    generation: u64,
}
impl PartialEq for Event {
    fn eq(&self, other: &Self) -> bool {
        self.deadline == other.deadline && self.sequence == other.sequence
    }
}
impl Eq for Event {}
impl PartialOrd for Event {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Event {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .deadline
            .cmp(&self.deadline)
            .then_with(|| other.sequence.cmp(&self.sequence))
    }
}
enum Command {
    Event(Event),
    Stop,
    Shutdown,
}
impl OutputQueue {
    pub fn new(output: SharedOutput) -> Self {
        let (tx, rx) = mpsc::channel::<Command>();
        let errors = Arc::new(Mutex::new(VecDeque::new()));
        let worker_errors = errors.clone();
        let worker = thread::spawn(move || {
            let mut heap = BinaryHeap::<Event>::new();
            let mut generation = 0_u64;
            let mut active = HashMap::<(i32, u8), u32>::new();
            loop {
                let command = if let Some(event) = heap.peek() {
                    let wait = event.deadline.saturating_duration_since(Instant::now());
                    match rx.recv_timeout(wait) {
                        Ok(command) => Some(command),
                        Err(mpsc::RecvTimeoutError::Timeout) => None,
                        Err(_) => break,
                    }
                } else {
                    match rx.recv() {
                        Ok(command) => Some(command),
                        Err(_) => break,
                    }
                };
                if let Some(command) = command {
                    match command {
                        Command::Event(event) => heap.push(event),
                        Command::Stop => {
                            generation += 1;
                            heap.clear();
                            let notes: Vec<_> = active.keys().copied().collect();
                            for (note, channel) in notes {
                                if let Err(error) =
                                    output.lock().unwrap().send("off", note, 0, channel)
                                {
                                    let mut errors = worker_errors.lock().unwrap();
                                    if errors.len() >= 32 {
                                        errors.pop_front();
                                    }
                                    errors.push_back(error.to_string());
                                }
                            }
                            let channels: Vec<_> =
                                active.keys().map(|(_, channel)| *channel).collect();
                            for channel in channels {
                                for (controller, value) in [(123, 0), (120, 0)] {
                                    if let Err(error) = output
                                        .lock()
                                        .unwrap()
                                        .control_change(controller, value, channel)
                                    {
                                        let mut errors = worker_errors.lock().unwrap();
                                        if errors.len() >= 32 {
                                            errors.pop_front();
                                        }
                                        errors.push_back(error.to_string());
                                    }
                                }
                            }
                            active.clear();
                        }
                        Command::Shutdown => break,
                    }
                } else if let Some(event) = heap.pop() {
                    if event.generation != generation {
                        continue;
                    }
                    if let Err(error) = output.lock().unwrap().send(
                        &event.kind,
                        event.note,
                        event.velocity,
                        event.channel,
                    ) {
                        let mut errors = worker_errors.lock().unwrap();
                        if errors.len() >= 32 {
                            errors.pop_front();
                        }
                        errors.push_back(error.to_string());
                    } else if event.kind == "on" {
                        *active.entry((event.note, event.channel)).or_insert(0) += 1;
                    } else if event.kind == "off" {
                        if let Some(count) = active.get_mut(&(event.note, event.channel)) {
                            *count = count.saturating_sub(1);
                            if *count == 0 {
                                active.remove(&(event.note, event.channel));
                            }
                        }
                    }
                }
            }
        });
        Self {
            tx,
            errors,
            generation: Arc::new(AtomicU64::new(0)),
            worker: Mutex::new(Some(worker)),
        }
    }
    pub fn send_at(
        &self,
        deadline: Instant,
        kind: String,
        note: i32,
        velocity: u8,
        channel: u8,
    ) -> Result<()> {
        let generation = self.generation.load(AtomicOrdering::Relaxed);
        self.tx
            .send(Command::Event(Event {
                deadline,
                sequence: next_sequence(),
                kind,
                note,
                velocity,
                channel,
                generation,
            }))
            .map_err(|e| anyhow!(e.to_string()))
    }
    pub fn stop_and_release(&self) {
        self.generation.fetch_add(1, AtomicOrdering::Relaxed);
        let _ = self.tx.send(Command::Stop);
    }
    pub fn shutdown(&self) {
        let _ = self.tx.send(Command::Shutdown);
        if let Some(worker) = self.worker.lock().unwrap().take() {
            let _ = worker.join();
        }
    }
    pub fn drain_errors(&self) -> Vec<String> {
        self.errors.lock().unwrap().drain(..).collect()
    }
}
impl Drop for OutputQueue {
    fn drop(&mut self) {
        self.shutdown();
    }
}
fn next_sequence() -> u64 {
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    SEQUENCE.fetch_add(1, AtomicOrdering::Relaxed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    #[derive(Clone)]
    struct Recording {
        events: Arc<Mutex<Vec<[u8; 3]>>>,
    }
    impl OutputBackend for Recording {
        fn send(&mut self, kind: &str, note: i32, velocity: u8, channel: u8) -> Result<()> {
            self.events
                .lock()
                .unwrap()
                .push(note_bytes(kind, note, velocity, channel)?);
            Ok(())
        }
        fn control_change(&mut self, controller: u8, value: u8, channel: u8) -> Result<()> {
            self.events
                .lock()
                .unwrap()
                .push([0xB0 | (channel - 1), controller, value]);
            Ok(())
        }
    }
    fn wait_for(events: &Arc<Mutex<Vec<[u8; 3]>>>, n: usize) {
        for _ in 0..50 {
            if events.lock().unwrap().len() >= n {
                return;
            }
            thread::sleep(Duration::from_millis(10));
        }
    }
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
    fn queue_delivers_actual_event() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let backend: SharedOutput = Arc::new(Mutex::new(Box::new(Recording {
            events: events.clone(),
        })));
        let q = OutputQueue::new(backend.clone());
        q.send_at(
            Instant::now() + Duration::from_millis(10),
            "on".into(),
            60,
            100,
            1,
        )
        .unwrap();
        wait_for(&events, 1);
        assert_eq!(events.lock().unwrap().as_slice(), &[[0x90, 60, 100]]);
    }
    #[test]
    fn queue_orders_deadlines_and_stable_ties() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let q = OutputQueue::new(Arc::new(Mutex::new(Box::new(Recording {
            events: events.clone(),
        }))));
        let now = Instant::now() + Duration::from_millis(30);
        q.send_at(now, "on".into(), 1, 1, 1).unwrap();
        q.send_at(now - Duration::from_millis(10), "on".into(), 2, 1, 1)
            .unwrap();
        q.send_at(now, "on".into(), 3, 1, 1).unwrap();
        wait_for(&events, 3);
        assert_eq!(
            events
                .lock()
                .unwrap()
                .iter()
                .map(|x| x[1])
                .collect::<Vec<_>>(),
            vec![2, 1, 3]
        );
    }
    #[test]
    fn queue_cancels_generation_and_joins() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let q = OutputQueue::new(Arc::new(Mutex::new(Box::new(Recording {
            events: events.clone(),
        }))));
        q.send_at(
            Instant::now() + Duration::from_millis(100),
            "on".into(),
            60,
            100,
            1,
        )
        .unwrap();
        q.stop_and_release();
        thread::sleep(Duration::from_millis(150));
        assert!(events.lock().unwrap().is_empty());
        q.shutdown();
    }
    #[test]
    fn stop_releases_active_notes_and_cancels_future_notes() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let q = OutputQueue::new(Arc::new(Mutex::new(Box::new(Recording {
            events: events.clone(),
        }))));
        q.send_at(Instant::now(), "on".into(), 60, 100, 1).unwrap();
        q.send_at(
            Instant::now() + Duration::from_millis(100),
            "on".into(),
            62,
            100,
            1,
        )
        .unwrap();
        wait_for(&events, 1);
        q.stop_and_release();
        thread::sleep(Duration::from_millis(150));
        let events = events.lock().unwrap();
        assert_eq!(&events[..2], &[[0x90, 60, 100], [0x80, 60, 0]]);
        assert!(events.contains(&[0xB0, 123, 0]));
        assert!(events.contains(&[0xB0, 120, 0]));
    }
}
