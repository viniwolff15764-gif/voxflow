use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::SampleFormat;
use std::io::Cursor;
use std::sync::{Arc, Mutex};

pub struct AudioRecorder {
    /// Full recording buffer. Never drained while recording, so the final
    /// transcription can re-process the whole audio in one shot (better accuracy).
    samples: Arc<Mutex<Vec<f32>>>,
    /// How many samples have already been handed out as live-preview chunks.
    preview_pos: usize,
    stream: Option<cpal::Stream>,
    sample_rate: u32,
    channels: u16,
}

// Safety: AudioRecorder lives behind a Mutex and the cpal::Stream is only ever
// created, played and dropped on the same (global-shortcut handler) thread.
// cpal::Stream is !Send on macOS/Windows, so we assert Send to share the
// recorder across the async preview task (which only touches `samples`).
unsafe impl Send for AudioRecorder {}

impl AudioRecorder {
    pub fn new() -> Result<Self, String> {
        let (sample_rate, channels) = default_input_format()?;
        Ok(Self {
            samples: Arc::new(Mutex::new(Vec::new())),
            preview_pos: 0,
            stream: None,
            sample_rate,
            channels,
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
        let sample_format = config.sample_format();

        // Reset buffers.
        self.samples.lock().unwrap().clear();
        self.preview_pos = 0;

        let samples = Arc::clone(&self.samples);
        let err_fn = |err| eprintln!("Audio stream error: {}", err);
        let stream_config: cpal::StreamConfig = config.into();

        // Build the stream for whatever sample format the device provides and
        // normalise everything to f32 in [-1.0, 1.0].
        let stream = match sample_format {
            SampleFormat::F32 => device.build_input_stream(
                &stream_config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    samples.lock().unwrap().extend_from_slice(data);
                },
                err_fn,
                None,
            ),
            SampleFormat::I16 => device.build_input_stream(
                &stream_config,
                move |data: &[i16], _: &cpal::InputCallbackInfo| {
                    let mut buf = samples.lock().unwrap();
                    buf.extend(data.iter().map(|&s| s as f32 / i16::MAX as f32));
                },
                err_fn,
                None,
            ),
            SampleFormat::U16 => device.build_input_stream(
                &stream_config,
                move |data: &[u16], _: &cpal::InputCallbackInfo| {
                    let mut buf = samples.lock().unwrap();
                    buf.extend(
                        data.iter()
                            .map(|&s| (s as f32 - u16::MAX as f32 / 2.0) / (u16::MAX as f32 / 2.0)),
                    );
                },
                err_fn,
                None,
            ),
            other => return Err(format!("Unsupported sample format: {:?}", other)),
        }
        .map_err(|e| format!("Failed to build stream: {}", e))?;

        stream
            .play()
            .map_err(|e| format!("Failed to play stream: {}", e))?;
        self.stream = Some(stream);
        Ok(())
    }

    pub fn stop(&mut self) {
        self.stream = None; // Drop stops the stream.
    }

    /// Return the samples recorded since the last preview call, encoded as WAV.
    /// Used only for the live preview while the user is still talking.
    pub fn take_preview_chunk(&mut self) -> Option<Vec<u8>> {
        let samples = self.samples.lock().unwrap();
        if samples.len() <= self.preview_pos {
            return None;
        }
        let chunk: Vec<f32> = samples[self.preview_pos..].to_vec();
        self.preview_pos = samples.len();
        drop(samples);
        Some(encode_wav(&chunk, self.sample_rate, self.channels))
    }

    /// Return the ENTIRE recording encoded as WAV, for the accurate final pass.
    pub fn take_all(&self) -> Option<Vec<u8>> {
        let data: Vec<f32> = {
            let samples = self.samples.lock().unwrap();
            if samples.is_empty() {
                return None;
            }
            samples.clone()
        };
        Some(encode_wav(&data, self.sample_rate, self.channels))
    }
}

fn default_input_format() -> Result<(u32, u16), String> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or("No input device found")?;
    let config = device
        .default_input_config()
        .map_err(|e| format!("No input config: {}", e))?;
    Ok((config.sample_rate().0, config.channels()))
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
        let clamped = sample.clamp(-1.0, 1.0);
        let amplitude = (clamped * i16::MAX as f32) as i16;
        writer.write_sample(amplitude).unwrap();
    }

    writer.finalize().unwrap();
    cursor.into_inner()
}
