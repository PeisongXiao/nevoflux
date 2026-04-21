//! Wire types for `jobs:render:{job_id}` EventBus events.
//!
//! The daemon emits payloads of shape
//! `{"event":"progress"|"succeeded"|"failed"|"cancelled", "job_id":…, …}`
//! — mirrored here as an enum so the sidebar can `match` on variant
//! rather than string-comparing an `event` field everywhere.

use serde::{Deserialize, Serialize};

/// The four possible terminal / in-flight states emitted on
/// `jobs:render:{job_id}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum RenderProgressEvent {
    Progress {
        job_id: String,
        current: u32,
        total: u32,
    },
    Succeeded {
        job_id: String,
        output_path: String,
        size_bytes: u64,
    },
    Failed {
        job_id: String,
        error: String,
    },
    Cancelled {
        job_id: String,
        current: u32,
        total: u32,
    },
}

impl RenderProgressEvent {
    /// Uniform accessor (all four variants carry a job_id).
    pub fn job_id(&self) -> &str {
        match self {
            Self::Progress { job_id, .. }
            | Self::Succeeded { job_id, .. }
            | Self::Failed { job_id, .. }
            | Self::Cancelled { job_id, .. } => job_id,
        }
    }
}

/// Simplified state for UI rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RenderJobState {
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_event_json_roundtrip() {
        let e = RenderProgressEvent::Progress {
            job_id: "job-x".into(),
            current: 42,
            total: 150,
        };
        let s = serde_json::to_string(&e).unwrap();
        assert!(s.contains(r#""event":"progress""#), "{}", s);
        assert_eq!(e, serde_json::from_str(&s).unwrap());
    }

    #[test]
    fn succeeded_event_json_roundtrip() {
        let e = RenderProgressEvent::Succeeded {
            job_id: "job-x".into(),
            output_path: "/tmp/out.mp4".into(),
            size_bytes: 12345,
        };
        let s = serde_json::to_string(&e).unwrap();
        assert!(s.contains(r#""event":"succeeded""#), "{}", s);
        assert_eq!(e, serde_json::from_str(&s).unwrap());
    }

    #[test]
    fn failed_event_json_roundtrip() {
        let e = RenderProgressEvent::Failed {
            job_id: "job-x".into(),
            error: "ffmpeg crashed".into(),
        };
        let s = serde_json::to_string(&e).unwrap();
        assert_eq!(e, serde_json::from_str(&s).unwrap());
    }

    #[test]
    fn cancelled_event_json_roundtrip() {
        let e = RenderProgressEvent::Cancelled {
            job_id: "job-x".into(),
            current: 7,
            total: 150,
        };
        let s = serde_json::to_string(&e).unwrap();
        assert_eq!(e, serde_json::from_str(&s).unwrap());
    }

    #[test]
    fn job_id_accessor_works_for_all_variants() {
        assert_eq!(
            RenderProgressEvent::Progress {
                job_id: "a".into(),
                current: 0,
                total: 0
            }
            .job_id(),
            "a"
        );
        assert_eq!(
            RenderProgressEvent::Failed {
                job_id: "b".into(),
                error: "".into()
            }
            .job_id(),
            "b"
        );
    }
}
