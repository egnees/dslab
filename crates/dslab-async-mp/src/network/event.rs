//! Definition of possible network events.

use serde::Serialize;

use super::{message::Message, tag::Tag};

/// Represents event when message is delivered to the destination.
#[derive(Clone, Serialize)]
pub struct MessageDelivered {
    /// Id of the delivered message.
    pub msg_id: u64,
    /// Delivered message.
    pub msg: Message,
    /// Source process.
    pub src_proc: String,
    /// Source node.
    pub src_node: String,
    /// Destination process.
    pub dst_proc: String,
    /// Destination node.
    pub dst_node: String,
}

/// Represents event when message is dropped between delivery.
#[derive(Clone, Serialize)]
pub struct MessageDropped {
    /// Id of the delivered message.
    pub msg_id: u64,
    /// Delivered message.
    pub msg: Message,
    /// Source process.
    pub src_proc: String,
    /// Source node.
    pub src_node: String,
    /// Destination process.
    pub dst_proc: String,
    /// Destination node.
    pub dst_node: String,
}

/// Represents event which is appeared when [`send_wisth_tag`] method of `Context` is called.
#[derive(Clone, Serialize)]
pub struct TaggedMessageDelivered {
    /// Id of the delivered message.
    pub msg_id: u64,
    /// Delivered message.
    pub msg: Message,
    /// Source process.
    pub src_proc: String,
    /// Source node.
    pub src_node: String,
    /// Destination process.
    pub dst_proc: String,
    /// Destination node.
    pub dst_node: String,
    /// Represents message tag.
    pub tag: Tag,
}

impl From<MessageDelivered> for MessageDropped {
    fn from(value: MessageDelivered) -> Self {
        Self {
            msg_id: value.msg_id,
            msg: value.msg,
            src_proc: value.src_proc,
            src_node: value.src_node,
            dst_proc: value.dst_proc,
            dst_node: value.dst_node,
        }
    }
}
