//! Definition of process events.

use serde::Serialize;

#[derive(Clone, Serialize)]
pub struct TimerFired {
    pub time: f64,
    pub name: String,
    pub node: String,
    pub proc: String,
}
