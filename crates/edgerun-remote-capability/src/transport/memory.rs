//! In-memory channel-pair transport for testing.

use crate::cell::RefCell;
use crate::collections::VecDeque;
use crate::prelude::v1::*;
use crate::rc::Rc;

use edgerun_capabilities::CapabilityError;
use edgerun_core::protocol::capability_runtime::CapabilityRemoteEnvelope;

use crate::protocol::RemoteCapabilityTransport;

#[derive(Clone, Debug)]
pub struct MemoryRemoteTransport {
    inbox: Rc<RefCell<VecDeque<CapabilityRemoteEnvelope>>>,
    outbox: Rc<RefCell<VecDeque<CapabilityRemoteEnvelope>>>,
}

impl MemoryRemoteTransport {
    pub fn pair() -> (Self, Self) {
        let a_to_b = Rc::new(RefCell::new(VecDeque::new()));
        let b_to_a = Rc::new(RefCell::new(VecDeque::new()));
        (
            Self {
                inbox: b_to_a.clone(),
                outbox: a_to_b.clone(),
            },
            Self {
                inbox: a_to_b,
                outbox: b_to_a,
            },
        )
    }
}

impl RemoteCapabilityTransport for MemoryRemoteTransport {
    fn send(&mut self, envelope: CapabilityRemoteEnvelope) -> Result<(), CapabilityError> {
        self.outbox.borrow_mut().push_back(envelope);
        Ok(())
    }

    fn recv(&mut self) -> Result<Option<CapabilityRemoteEnvelope>, CapabilityError> {
        Ok(self.inbox.borrow_mut().pop_front())
    }
}
