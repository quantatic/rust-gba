use std::collections::VecDeque;

const BUFFER_SIZE: usize = 32;

#[derive(Clone, Debug, Default)]
pub(super) struct DmaFifo {
    buffer: VecDeque<i8>,
}

impl DmaFifo {
    pub(super) fn sample(&self) -> i8 {
        self.buffer.front().copied().unwrap_or(0)
    }

    // Should be called when the timer configured for this APU queue expires.
    // In this case, the next sample should be fed to the APU mixer.
    pub(super) fn timer_expired(&mut self) {
        self.buffer.pop_front();
    }

    pub(super) fn write_data(&mut self, data: u32) {
        for byte in data.to_le_bytes() {
            // Drop any samples that attempt to write beyond the buffer.
            if self.buffer.len() < BUFFER_SIZE {
                self.buffer.push_back(byte as i8);
            }
        }
    }
}
