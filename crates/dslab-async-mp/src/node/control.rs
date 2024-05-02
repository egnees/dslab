//! Definition of block for interacting with simulation.

use std::{cell::RefCell, rc::Rc};

use dslab_core::SimulationContext;

use crate::{log::logger::Logger, network::model::Network, storage::file_manager::FileManager};

///! Definition of control block.

/// Represents collection of the elements, which are used to interact with simulation.
/// Every node owns its own [`ControlBlock`].
/// [`ControlBlock`] is shared between different copies of context.
pub struct ControlBlock {
    /// Represents simulation network.
    pub network: Rc<RefCell<Network>>,
    /// Manages file in storage.
    pub file_manager: FileManager,
    /// Simulation logger.
    pub logger: Rc<RefCell<Logger>>,
    /// Context of node.
    pub ctx: SimulationContext,
    /// Clock skew of node.
    pub clock_skew: f64,
    /// Name of the node.
    pub node_name: String,
}
