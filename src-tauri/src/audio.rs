use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::io::Cursor;
use std::sync::{Arc, Mutex};

pub struct AudioRecorder {
    samples: Arc<Mutex<Vec<f32>>>,
    stream: Option<cpal::Stream>,
    sample_rate: u32,
    channels: u16,
}

// Safety: AudioRecorder is always accessed behind a Mutex.
// cpal::Stream is not Send on all platforms but is safe behind Mutex on Windows (our target).
unsafe impl Send for AudioRecorder {}

impl AudioRecorder {
    pub fn new() -> Result<Self, String> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or("No input device found")?;

        let config = device
            .default_input_config()
            .map_err(|e| format!("No input config: {}", e))?;

        Ok(Self {
            samples: Arc::new(Mutex::new(Vec::new())),
            stream: None,
            sample_rate: config.sample_rate().0,
            channels: config.channels(),
        })
    }

    pub fn start(&mut self) -> Result<(), String> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or("No input device found")?;

        let config = device
            .default_input_config()
            .map_err(|e| format!("No input config: {}", e))?;

        self.sample_rate = config.sample_rate().0;
        self.channels = config.channels();

        // Clear previous samples
        self.samples.lock().unwrap().clear();

        let samples = Arc::clone(&self.samples);

        let stream = device
            .build_input_stream(
                &config.into(),
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    samples.lock().unwrap().extend_from_slice(data);
                },
                |err| {
                    eprintln!("Audio stream error: {}", err);
                },
                None,
            )
            .map_err(|e| format!("Failed to build stream: {}", e))?;

        stream.play().map_err(|e| format!("Failed to play stream: {}", e))?;
        self.stream = Some(stream);
        Ok(())
    }

    pub fn stop(&mut self) {
        self.stream = None; // Drop stops the stream
    }

    /// Take a chunk of samples recorded since last call, encode as WAV bytes.
    pub fn take_chunk(&self) -> Option<Vec<u8>> {
        let mut samples = self.samples.lock().unwrap();
        if samples.is_empty() {
            return None;
        }

        let chunk: Vec<f32> = samples.drain(..).collect();
        drop(samples);

        Some(encode_wav(&chunk, self.sample_rate, self.channels))
    }

    /// Take ALL recorded samples and encode as WAV.
    pub fn take_all(&self) -> Option<Vec<u8>> {
        self.take_chunk()
    }
}

fn encode_wav(samples: &[f32], sample_rate: u32, channels: u16) -> Vec<u8> {
    let mut cursor = Cursor::new(Vec::new());

    let spec = hound::WavSpec {
        channels,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = hound::WavWriter::new(&mut cursor, spec).unwrap();

    for &sample in samples {
        let amplitude = (sample * i16::MAX as f32) as i16;
        writer.write_sample(amplitude).unwrap();
    }

    writer.finalize().unwrap();
    cursor.into_inner()
}
