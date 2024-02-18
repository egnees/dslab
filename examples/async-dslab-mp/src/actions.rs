//! Represents process actions.

use serde::Serialize;

use crate::message::Message;

/// For now there is only local message sent action.
/// It future, there can be timer set action and others.
#[derive(Clone, Serialize)]
pub struct LocalMessageSentAction {
    pub msg: Message,
}
