//! Process trait and related types.

use super::context::Context;
use crate::network::message::Message;

pub trait Process {
    /// Called when a message is received.
    fn on_message(&mut self, msg: Message, from: String, ctx: Context) -> Result<(), String>;

    /// Called when a _local_ message is received.
    fn on_local_message(&mut self, msg: Message, ctx: Context) -> Result<(), String>;

    /// Called when a timer fires.
    fn on_timer(&mut self, timer: String, ctx: Context) -> Result<(), String>;
}
