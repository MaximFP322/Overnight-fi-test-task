use crate::queue::QueueSendError;

use super::ConnectorOut;

pub trait ConnectorOutbox {
    fn try_send(&mut self, outgoing: ConnectorOut) -> Result<(), QueueSendError>;
}
