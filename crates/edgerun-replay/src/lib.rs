//! Layer 17: Deterministic Replay Engine
//!
//! Record a rendering session as a sequence of proto state changes,
//! replay it byte-for-byte identical. Bug reproduction: user reports
//! "my page looks wrong" → they export the replay file → you replay
//! it locally → exact same rendering. No "works on my machine."

use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};

/// Binary format for replay recordings.
///
/// Header: b"EDGERUN\0" (8 bytes)
/// Version: u32 (4 bytes)
/// Frame count: u32 (4 bytes)
/// Frame records: [FrameRecord]
///
/// FrameRecord:
///   Frame length: u32 (4 bytes)
///   Changes: [ChangeRecord]
///
/// ChangeRecord:
///   Change type: u8 (1 byte)
///   Node index: u32 (4 bytes)
///   Property index: u16 (2 bytes) — proto enum discriminant
///   Value length: u16 (2 bytes)
///   Value: [u8] (variable)

const MAGIC: &[u8] = b"EDGERUN\0";
const VERSION: u32 = 1;

/// A single state change in a replay session.
#[derive(Clone, Debug)]
pub struct ChangeRecord {
    pub node_idx: u32,
    pub property_idx: u16,  // proto enum discriminant
    pub value: Vec<u8>,
}

/// A frame in a replay session — all changes from one render pass.
#[derive(Clone, Debug)]
pub struct FrameRecord {
    pub changes: Vec<ChangeRecord>,
}

/// A complete replay recording.
#[derive(Clone, Debug)]
pub struct ReplayRecording {
    pub frames: Vec<FrameRecord>,
}

impl ReplayRecording {
    /// Create a new empty recording.
    pub fn new() -> Self {
        Self { frames: Vec::new() }
    }

    /// Add a frame with changes.
    pub fn add_frame(&mut self, changes: Vec<ChangeRecord>) {
        self.frames.push(FrameRecord { changes });
    }

    /// Save to a binary file.
    pub fn save(&self, path: &str) -> std::io::Result<()> {
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);

        // Header
        writer.write_all(MAGIC)?;
        writer.write_all(&VERSION.to_le_bytes())?;
        writer.write_all(&(self.frames.len() as u32).to_le_bytes())?;

        // Frames
        for frame in &self.frames {
            // Frame length (in bytes of changes data)
            let mut frame_data = Vec::new();
            for change in &frame.changes {
                frame_data.push(0x01); // ChangeRecord type
                frame_data.extend_from_slice(&change.node_idx.to_le_bytes());
                frame_data.extend_from_slice(&change.property_idx.to_le_bytes());
                frame_data.extend_from_slice(&(change.value.len() as u16).to_le_bytes());
                frame_data.extend_from_slice(&change.value);
            }
            writer.write_all(&(frame_data.len() as u32).to_le_bytes())?;
            writer.write_all(&frame_data)?;
        }

        writer.flush()?;
        Ok(())
    }

    /// Load from a binary file.
    pub fn load(path: &str) -> std::io::Result<Self> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);

        // Verify header
        let mut magic = [0u8; 8];
        reader.read_exact(&mut magic)?;
        if magic != MAGIC {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid replay file magic"));
        }

        let mut version_bytes = [0u8; 4];
        reader.read_exact(&mut version_bytes)?;
        let version = u32::from_le_bytes(version_bytes);
        if version != VERSION {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData,
                format!("Unsupported version: {}", version)));
        }

        let mut frame_count_bytes = [0u8; 4];
        reader.read_exact(&mut frame_count_bytes)?;
        let frame_count = u32::from_le_bytes(frame_count_bytes);

        let mut frames = Vec::with_capacity(frame_count as usize);
        for _ in 0..frame_count {
            let mut len_bytes = [0u8; 4];
            reader.read_exact(&mut len_bytes)?;
            let frame_len = u32::from_le_bytes(len_bytes) as usize;

            let mut frame_data = vec![0u8; frame_len];
            reader.read_exact(&mut frame_data)?;

            let changes = parse_changes(&frame_data)?;
            frames.push(FrameRecord { changes });
        }

        Ok(Self { frames })
    }

    /// Get total number of changes across all frames.
    pub fn total_changes(&self) -> usize {
        self.frames.iter().map(|f| f.changes.len()).sum()
    }

    /// Get total byte size of the recording.
    pub fn byte_size(&self) -> usize {
        8 + 4 + 4 + self.frames.iter().map(|f| {
            4 + f.changes.iter().map(|c| 1 + 4 + 2 + 2 + c.value.len()).sum::<usize>()
        }).sum::<usize>()
    }
}

fn parse_changes(data: &[u8]) -> std::io::Result<Vec<ChangeRecord>> {
    let mut changes = Vec::new();
    let mut pos = 0;

    while pos < data.len() {
        if data[pos] != 0x01 {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid change record type"));
        }
        pos += 1;

        if pos + 4 > data.len() { return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Truncated node_idx")); }
        let node_idx = u32::from_le_bytes([data[pos], data[pos+1], data[pos+2], data[pos+3]]);
        pos += 4;

        if pos + 2 > data.len() { return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Truncated property_idx")); }
        let property_idx = u16::from_le_bytes([data[pos], data[pos+1]]);
        pos += 2;

        if pos + 2 > data.len() { return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Truncated value_len")); }
        let value_len = u16::from_le_bytes([data[pos], data[pos+1]]) as usize;
        pos += 2;

        if pos + value_len > data.len() { return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Truncated value")); }
        let value = data[pos..pos + value_len].to_vec();
        pos += value_len;

        changes.push(ChangeRecord { node_idx, property_idx, value });
    }

    Ok(changes)
}

/// A replay player that applies recorded changes to a renderer.
pub struct ReplayPlayer {
    recording: ReplayRecording,
    current_frame: usize,
}

impl ReplayPlayer {
    pub fn new(recording: ReplayRecording) -> Self {
        Self { recording, current_frame: 0 }
    }

    /// Load from file and create a player.
    pub fn from_file(path: &str) -> std::io::Result<Self> {
        let recording = ReplayRecording::load(path)?;
        Ok(Self::new(recording))
    }

    /// Get the next frame's changes, or None if done.
    pub fn next_frame(&mut self) -> Option<&FrameRecord> {
        if self.current_frame >= self.recording.frames.len() {
            return None;
        }
        let frame = &self.recording.frames[self.current_frame];
        self.current_frame += 1;
        Some(frame)
    }

    /// Reset to the beginning.
    pub fn reset(&mut self) {
        self.current_frame = 0;
    }

    /// Total frames in the recording.
    pub fn frame_count(&self) -> usize {
        self.recording.frames.len()
    }

    /// Current frame index.
    pub fn current_frame(&self) -> usize {
        self.current_frame
    }
}

/// A replay recorder that captures state changes.
pub struct ReplayRecorder {
    recording: ReplayRecording,
    pending_changes: Vec<ChangeRecord>,
}

impl ReplayRecorder {
    pub fn new() -> Self {
        Self {
            recording: ReplayRecording::new(),
            pending_changes: Vec::new(),
        }
    }

    /// Record a DOM change (node, property, value).
    pub fn record_dom_change(&mut self, node_idx: u32, property_idx: u16, value: &[u8]) {
        self.pending_changes.push(ChangeRecord {
            node_idx,
            property_idx,
            value: value.to_vec(),
        });
    }

    /// Record a CSS rule change.
    pub fn record_css_rule_change(&mut self, rule_idx: u32, declaration: &str, value: &str) {
        let value_bytes = format!("{}:{}", declaration, value).into_bytes();
        self.pending_changes.push(ChangeRecord {
            node_idx: rule_idx,
            property_idx: 0xFFFF, // Special marker for rule changes
            value: value_bytes,
        });
    }

    /// Finish the current frame and start a new one.
    pub fn end_frame(&mut self) {
        let changes = std::mem::take(&mut self.pending_changes);
        self.recording.add_frame(changes);
    }

    /// Finish recording and save to file.
    pub fn save(self, path: &str) -> std::io::Result<ReplayRecording> {
        let recording = self.recording;
        recording.save(path)?;
        Ok(recording)
    }

    /// Get the recording without saving.
    pub fn finish(self) -> ReplayRecording {
        self.recording
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_record_and_playback() {
        let mut recorder = ReplayRecorder::new();
        recorder.record_dom_change(0, 60, b"32px"); // font-size
        recorder.record_dom_change(1, 0, b"#FF0000"); // color
        recorder.end_frame();

        recorder.record_dom_change(2, 44, b"flex"); // display
        recorder.end_frame();

        let recording = recorder.finish();
        assert_eq!(recording.frames.len(), 2);
        assert_eq!(recording.frames[0].changes.len(), 2);
        assert_eq!(recording.frames[1].changes.len(), 1);

        // Save and reload
        let path = "/tmp/test_replay.bin";
        recording.save(path).unwrap();
        let loaded = ReplayRecording::load(path).unwrap();
        fs::remove_file(path).ok();

        assert_eq!(loaded.frames.len(), 2);
        assert_eq!(loaded.frames[0].changes[0].node_idx, 0);
        assert_eq!(loaded.frames[0].changes[0].property_idx, 60);
        assert_eq!(loaded.frames[0].changes[0].value, b"32px");
    }

    #[test]
    fn test_player_iteration() {
        let mut recorder = ReplayRecorder::new();
        recorder.record_dom_change(0, 60, b"16px");
        recorder.end_frame();
        recorder.record_dom_change(0, 60, b"24px");
        recorder.end_frame();

        let recording = recorder.finish();
        let mut player = ReplayPlayer::new(recording);

        // Frame 0
        let frame = player.next_frame().unwrap();
        assert_eq!(frame.changes.len(), 1);
        assert_eq!(frame.changes[0].value, b"16px");

        // Frame 1
        let frame = player.next_frame().unwrap();
        assert_eq!(frame.changes[0].value, b"24px");

        // Done
        assert!(player.next_frame().is_none());
        assert_eq!(player.current_frame(), 2);
    }

    #[test]
    fn test_reset() {
        let mut recorder = ReplayRecorder::new();
        recorder.record_dom_change(0, 1, b"test");
        recorder.end_frame();

        let recording = recorder.finish();
        let mut player = ReplayPlayer::new(recording);

        player.next_frame();
        assert_eq!(player.current_frame(), 1);

        player.reset();
        assert_eq!(player.current_frame(), 0);

        let frame = player.next_frame().unwrap();
        assert_eq!(frame.changes[0].value, b"test");
    }

    #[test]
    fn test_byte_size() {
        let mut recorder = ReplayRecorder::new();
        recorder.record_dom_change(0, 60, b"32px");
        recorder.end_frame();

        let recording = recorder.finish();
        let size = recording.byte_size();
        // Header (16) + frame len (4) + change (1+4+2+2+4) = 33
        assert_eq!(size, 33);
    }

    #[test]
    fn test_invalid_magic() {
        let path = "/tmp/test_bad_magic.bin";
        fs::write(path, b"BAD MAGIC!").unwrap();
        let result = ReplayRecording::load(path);
        fs::remove_file(path).ok();
        assert!(result.is_err());
    }
}
