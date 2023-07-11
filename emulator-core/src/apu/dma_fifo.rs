use std::collections::VecDeque;

use crate::CYCLES_PER_SECOND;

// Number of 32-bit samples.
const BUFFER_SIZE: usize = 32;

const SAMPLE_FREQUENCY: u64 = 32_768;

#[derive(Clone, Debug, Default)]
pub(super) struct DmaFifo {
    buffer: VecDeque<i8>,

    wants_dma: bool,
}

impl DmaFifo {
    pub fn step(&mut self, timer_overflow: bool) {
        if timer_overflow {
            let pop_result = self.buffer.pop_front();

            if pop_result.is_none() {
                // log::error!("attempted to step dma fifo with empty buffer");
            }

            if self.buffer.len() <= 16 {
                self.wants_dma = true;
            }
        }
    }

    // 0 to 255 inclusive.
    pub fn sample(&self) -> u8 {
        let Some(current_sample) = self.buffer.front().copied() else {
            return 0;
        };

        let result = if current_sample == i8::MIN {
            0
        } else if current_sample < 0 {
            128 - ((-current_sample) as u8)
        } else {
            (current_sample as u8) + 128
        };
        result
    }

    pub(super) fn write_data(&mut self, data: u32) {
        for byte in data.to_le_bytes() {
            // Drop any samples that attempt to write beyond the buffer.
            if self.buffer.len() < BUFFER_SIZE {
                self.buffer.push_back(byte as i8);
            } else {
                log::error!("attempted to write data beyond the dma fifo buffer");
            }
        }
    }

    pub fn poll_wants_dma(&mut self) -> bool {
        let result = self.wants_dma;
        self.wants_dma = false;
        result
    }
}
