#![allow(dead_code)]
use crate::project::Pattern;
use anyhow::{anyhow, Result};
use rosc::{OscMessage, OscPacket, OscType};

pub const ROOT: &str = "/index/v1";
pub const READY: &str = "/index/v1/ready";
pub const SYNC: &str = "/index/v1/project/sync";
pub const SYNC_ACK: &str = "/index/v1/project/sync_ack";
pub const STAGE: &str = "/index/v1/pattern/set";
pub const STAGED: &str = "/index/v1/pattern/staged";
pub const COMMIT: &str = "/index/v1/pattern/commit";
pub const COMMITTED: &str = "/index/v1/pattern/committed";
pub const EVENT: &str = "/index/v1/event/midi";
pub const ERROR: &str = "/index/v1/error";
pub const CANCELLED: &str = "/index/v1/request/cancelled";
pub const SUPERSEDED: &str = "/index/v1/request/superseded";

pub fn strings(xs: &[OscType], start: usize) -> Result<Vec<String>> {
    xs.get(start..)
        .unwrap_or_default()
        .iter()
        .map(|x| match x {
            OscType::String(v) => Ok(v.clone()),
            _ => Err(anyhow!("expected OSC string")),
        })
        .collect()
}
pub fn one_string(args: &[OscType], n: usize) -> Result<String> {
    match args.get(n) {
        Some(OscType::String(s)) => Ok(s.clone()),
        _ => Err(anyhow!("argument {n} must be a string")),
    }
}
pub fn one_int(args: &[OscType], n: usize) -> Result<i32> {
    match args.get(n) {
        Some(OscType::Int(x)) => Ok(*x),
        _ => Err(anyhow!("argument {n} must be an integer")),
    }
}
pub fn one_float(args: &[OscType], n: usize) -> Result<f32> {
    match args.get(n) {
        Some(OscType::Float(x)) => Ok(*x),
        Some(OscType::Double(x)) => Ok(*x as f32),
        _ => Err(anyhow!("argument {n} must be a float")),
    }
}
pub fn csv<T: std::str::FromStr>(s: &str) -> Result<Vec<T>> {
    s.split(',')
        .map(|x| {
            x.parse()
                .map_err(|_| anyhow!("malformed CSV pattern value"))
        })
        .collect()
}
pub fn pattern_args(request: &str, revision: u64, p: &Pattern) -> Vec<OscType> {
    vec![
        OscType::String(request.into()),
        OscType::Int(revision as i32),
        OscType::String(p.name.clone()),
        OscType::String(
            p.degrees
                .iter()
                .map(i32::to_string)
                .collect::<Vec<_>>()
                .join(","),
        ),
        OscType::String(
            p.durations
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(","),
        ),
        OscType::String(
            p.velocities
                .iter()
                .map(u8::to_string)
                .collect::<Vec<_>>()
                .join(","),
        ),
        OscType::Float(p.gate),
        OscType::Int(p.channel as i32),
        OscType::String(p.destination.clone()),
    ]
}
pub fn packet(path: &str, args: Vec<OscType>) -> OscPacket {
    OscPacket::Message(OscMessage {
        addr: path.into(),
        args,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_malformed() {
        assert!(csv::<i32>("1,nope").is_err());
        assert!(one_int(&[OscType::String("x".into())], 0).is_err());
    }
    #[test]
    fn correlation_is_encoded() {
        let p = crate::project::example_project(".").pattern;
        let a = pattern_args("req-1", 7, &p);
        assert_eq!(a[0], OscType::String("req-1".into()));
        assert_eq!(a[1], OscType::Int(7));
    }
}
