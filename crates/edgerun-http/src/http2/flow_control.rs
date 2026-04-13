//! HTTP/2 flow control (RFC 7540 Section 5.2)

use super::{Http2Error, Result};

/// Flow controller for connection or stream level
pub struct FlowController {
    /// Current window size
    window_size: i64,
    /// Initial window size
    initial_window_size: u32,
    /// Maximum window size (2^31 - 1)
    max_window_size: i64,
}

impl FlowController {
    /// Maximum allowed window size (2^31 - 1)
    pub const MAX_WINDOW_SIZE: i64 = 2147483647;

    /// Default initial window size (65535)
    pub const DEFAULT_INITIAL_WINDOW_SIZE: u32 = 65535;

    /// Create a new flow controller
    pub fn new(initial_window_size: u32) -> Self {
        FlowController {
            window_size: initial_window_size as i64,
            initial_window_size,
            max_window_size: Self::MAX_WINDOW_SIZE,
        }
    }

    /// Get current window size
    pub fn window_size(&self) -> i64 {
        self.window_size
    }

    /// Get initial window size
    pub fn initial_window_size(&self) -> u32 {
        self.initial_window_size
    }

    /// Consume window size (when sending data)
    pub fn consume(&mut self, size: u32) -> Result<()> {
        self.window_size -= size as i64;
        if self.window_size < 0 {
            return Err(Http2Error::FlowControl(format!(
                "Window size went negative: {}",
                self.window_size
            )));
        }
        Ok(())
    }

    /// Increment window size (when receiving WINDOW_UPDATE)
    pub fn increment(&mut self, increment: u32) -> Result<()> {
        // RFC 7540 §6.9: A receiver MUST treat a WINDOW_UPDATE frame
        // with an increment of 0 as a stream error (connection error if stream_id=0).
        if increment == 0 {
            return Err(Http2Error::FlowControl(
                "WINDOW_UPDATE with increment of 0".into(),
            ));
        }
        let new_size = self.window_size + increment as i64;
        if new_size > self.max_window_size {
            return Err(Http2Error::FlowControl(format!(
                "Window size would exceed maximum: {} + {} > {}",
                self.window_size, increment, self.max_window_size
            )));
        }
        self.window_size = new_size;
        Ok(())
    }

    /// Check if we can send data of given size
    pub fn can_send(&self, size: u32) -> bool {
        self.window_size >= size as i64
    }

    /// Get the maximum data we can send
    pub fn available(&self) -> u32 {
        if self.window_size <= 0 {
            0
        } else {
            self.window_size as u32
        }
    }

    /// Update initial window size (affects existing streams)
    pub fn update_initial_window_size(&mut self, new_size: u32) -> Result<()> {
        let delta = new_size as i64 - self.initial_window_size as i64;
        self.initial_window_size = new_size;

        // Adjust current window by delta
        let new_window = self.window_size + delta;
        if new_window < 0 || new_window > self.max_window_size {
            return Err(Http2Error::FlowControl(format!(
                "Window size update would result in invalid size: {}",
                new_window
            )));
        }

        self.window_size = new_window;
        Ok(())
    }

    /// Reset window size (after SETTINGS acknowledgment)
    pub fn reset(&mut self) {
        self.window_size = self.initial_window_size as i64;
    }
}

/// Manages flow control for multiple streams
pub struct FlowControlManager {
    /// Connection-level flow controller
    connection_flow: FlowController,
    /// Stream-level flow controllers
    stream_flows: std::collections::HashMap<u32, FlowController>,
}

impl FlowControlManager {
    /// Create a new flow control manager
    pub fn new(initial_window_size: u32) -> Self {
        FlowControlManager {
            connection_flow: FlowController::new(initial_window_size),
            stream_flows: std::collections::HashMap::new(),
        }
    }

    /// Add a new stream
    pub fn add_stream(&mut self, stream_id: u32) {
        self.stream_flows
            .insert(stream_id, FlowController::new(self.connection_flow.initial_window_size()));
    }

    /// Remove a stream
    pub fn remove_stream(&mut self, stream_id: u32) {
        self.stream_flows.remove(&stream_id);
    }

    /// Get connection-level window
    pub fn connection_window(&self) -> i64 {
        self.connection_flow.window_size()
    }

    /// Get stream-level window
    pub fn stream_window(&self, stream_id: u32) -> Option<i64> {
        self.stream_flows.get(&stream_id).map(|f| f.window_size())
    }

    /// Consume connection-level window
    pub fn consume_connection(&mut self, size: u32) -> Result<()> {
        self.connection_flow.consume(size)
    }

    /// Consume stream-level window
    pub fn consume_stream(&mut self, stream_id: u32, size: u32) -> Result<()> {
        if let Some(flow) = self.stream_flows.get_mut(&stream_id) {
            flow.consume(size)
        } else {
            Err(Http2Error::FlowControl(format!(
                "Stream {} not found",
                stream_id
            )))
        }
    }

    /// Update connection-level window
    pub fn increment_connection(&mut self, increment: u32) -> Result<()> {
        self.connection_flow.increment(increment)
    }

    /// Update stream-level window
    pub fn increment_stream(&mut self, stream_id: u32, increment: u32) -> Result<()> {
        if let Some(flow) = self.stream_flows.get_mut(&stream_id) {
            flow.increment(increment)
        } else {
            Err(Http2Error::FlowControl(format!(
                "Stream {} not found",
                stream_id
            )))
        }
    }

    /// Check if we can send data on a stream
    pub fn can_send_on_stream(&self, stream_id: u32, size: u32) -> bool {
        self.connection_flow.can_send(size)
            && self
                .stream_flows
                .get(&stream_id)
                .map(|f| f.can_send(size))
                .unwrap_or(false)
    }

    /// Update initial window size for all streams
    pub fn update_initial_window_size(&mut self, new_size: u32) -> Result<()> {
        self.connection_flow.update_initial_window_size(new_size)?;
        for flow in self.stream_flows.values_mut() {
            flow.update_initial_window_size(new_size)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flow_controller_new() {
        let fc = FlowController::new(65535);
        assert_eq!(fc.window_size(), 65535);
        assert_eq!(fc.initial_window_size(), 65535);
    }

    #[test]
    fn test_flow_controller_consume() {
        let mut fc = FlowController::new(1000);
        assert!(fc.consume(500).is_ok());
        assert_eq!(fc.window_size(), 500);

        assert!(fc.consume(500).is_ok());
        assert_eq!(fc.window_size(), 0);

        assert!(fc.consume(1).is_err());
    }

    #[test]
    fn test_flow_controller_increment() {
        let mut fc = FlowController::new(1000);
        fc.consume(1000).unwrap();
        assert_eq!(fc.window_size(), 0);

        fc.increment(500).unwrap();
        assert_eq!(fc.window_size(), 500);
    }

    #[test]
    fn test_flow_controller_can_send() {
        let mut fc = FlowController::new(1000);
        assert!(fc.can_send(500));
        assert!(fc.can_send(1000));
        assert!(!fc.can_send(1001));

        fc.consume(1000).unwrap();
        assert!(!fc.can_send(1));
    }

    #[test]
    fn test_flow_controller_available() {
        let mut fc = FlowController::new(1000);
        assert_eq!(fc.available(), 1000);

        fc.consume(600).unwrap();
        assert_eq!(fc.available(), 400);

        fc.consume(400).unwrap();
        assert_eq!(fc.available(), 0);
    }

    #[test]
    fn test_flow_controller_increment_overflow() {
        let mut fc = FlowController::new(1000);
        let result = fc.increment(FlowController::MAX_WINDOW_SIZE as u32);
        assert!(result.is_err());
    }

    #[test]
    fn test_flow_controller_update_initial() {
        let mut fc = FlowController::new(1000);
        fc.consume(500).unwrap();
        assert_eq!(fc.window_size(), 500);

        fc.update_initial_window_size(2000).unwrap();
        assert_eq!(fc.window_size(), 1500); // 500 + (2000 - 1000)
        assert_eq!(fc.initial_window_size(), 2000);
    }

    #[test]
    fn test_flow_control_manager() {
        let mut fcm = FlowControlManager::new(65535);

        // Add streams
        fcm.add_stream(1);
        fcm.add_stream(3);

        // Check windows
        assert_eq!(fcm.connection_window(), 65535);
        assert_eq!(fcm.stream_window(1), Some(65535));
        assert_eq!(fcm.stream_window(3), Some(65535));
        assert_eq!(fcm.stream_window(5), None);

        // Consume
        fcm.consume_stream(1, 1000).unwrap();
        assert_eq!(fcm.stream_window(1), Some(64535));

        // Increment
        fcm.increment_stream(1, 500).unwrap();
        assert_eq!(fcm.stream_window(1), Some(65035));

        // Remove stream
        fcm.remove_stream(1);
        assert_eq!(fcm.stream_window(1), None);
    }

    #[test]
    fn test_flow_control_manager_can_send() {
        let mut fcm = FlowControlManager::new(1000);
        fcm.add_stream(1);

        assert!(fcm.can_send_on_stream(1, 500));
        assert!(!fcm.can_send_on_stream(1, 1001));
        assert!(!fcm.can_send_on_stream(99, 500)); // Stream doesn't exist
    }
}
