use std::sync::mpsc::{channel, Receiver, Sender};

use rodio::Source;

pub struct SampleSource {
    receiver: Receiver<f32>,
    sample_rate: u32,
}

pub struct SampleSourceSender {
    sender: Sender<f32>,
}

pub fn sample_source(sample_rate: u32) -> (SampleSourceSender, SampleSource) {
    let (sender, receiver) = channel();

    let sample_source_sender = SampleSourceSender { sender };

    let sample_source = SampleSource {
        receiver,
        sample_rate,
    };

    (sample_source_sender, sample_source)
}

impl SampleSourceSender {
    pub fn push(&mut self, sample: f32) {
        self.sender.send(sample).unwrap();
    }
}

impl Source for SampleSource {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        2
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<std::time::Duration> {
        None
    }
}

impl Iterator for SampleSource {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        self.receiver.recv().ok()
    }
}
