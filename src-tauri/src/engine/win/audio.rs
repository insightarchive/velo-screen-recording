//! System sound (WASAPI loopback) and microphone capture, mixed on a steady clock and fed to the encoder.

use super::Shared;
use crate::engine::LevelsSink;
use parking_lot::Mutex;
use std::collections::VecDeque;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use velo_core::{Levels, Mixer, StartConfig, SAMPLE_RATE};
use wasapi::{initialize_mta, DeviceEnumerator, Direction, SampleType, StreamMode, WaveFormat};

pub struct Audio {
    threads: Vec<JoinHandle<()>>,
}

impl Audio {
    /// Waits for every audio thread to finish. Call after setting `Shared::stop`.
    pub fn join(self) {
        for t in self.threads {
            let _ = t.join();
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum Source {
    System,
    Mic(usize),
}

pub fn start(cfg: &StartConfig, shared: Arc<Shared>, levels: LevelsSink) -> Audio {
    let mixer = Arc::new(Mutex::new(Mixer::new(1.0, cfg.mic_gain.min(100) as f32 / 100.0)));
    let mut threads = Vec::new();

    let mut spawn_source = |source: Source| {
        let (shared, mixer) = (shared.clone(), mixer.clone());
        threads.push(thread::spawn(move || {
            if let Err(e) = capture_source(source, &shared, &mixer) {
                eprintln!("audio {source:?} stopped: {e}");
            }
        }));
    };
    if cfg.system {
        spawn_source(Source::System);
    }
    if cfg.mic {
        spawn_source(Source::Mic(cfg.mic_device));
    }

    threads.push(thread::spawn(move || drive(&shared, &mixer, &levels)));
    Audio { threads }
}

/// Reads one device and pushes 48 kHz stereo f32 into the mixer until recording stops.
fn capture_source(source: Source, shared: &Shared, mixer: &Mutex<Mixer>) -> Result<(), String> {
    // COM may already be initialised on this thread; that is fine.
    let _ = initialize_mta();
    let e = |x: wasapi::WasapiError| x.to_string();

    let enumerator = DeviceEnumerator::new().map_err(e)?;
    let device = match source {
        Source::System => enumerator.get_default_device(&Direction::Render).map_err(e)?,
        Source::Mic(0) => enumerator.get_default_device(&Direction::Capture).map_err(e)?,
        Source::Mic(n) => enumerator
            .get_device_collection(&Direction::Capture)
            .map_err(e)?
            .get_device_at_index((n - 1) as u32)
            .map_err(e)?,
    };

    let mut client = device.get_iaudioclient().map_err(e)?;
    let format = WaveFormat::new(32, 32, &SampleType::Float, SAMPLE_RATE as usize, 2, None);
    let (_, min_period) = client.get_device_period().map_err(e)?;
    // Asking a *render* device for Capture is how WASAPI does loopback.
    let mode = StreamMode::EventsShared { autoconvert: true, buffer_duration_hns: min_period };
    client.initialize_client(&format, &Direction::Capture, &mode).map_err(e)?;
    let event = client.set_get_eventhandle().map_err(e)?;
    let capture = client.get_audiocaptureclient().map_err(e)?;
    client.start_stream().map_err(e)?;

    let mut bytes: VecDeque<u8> = VecDeque::new();
    while !shared.stop.load(Ordering::Relaxed) {
        // Loopback delivers nothing while the PC is silent, so a timeout is normal.
        if event.wait_for_event(100).is_err() {
            continue;
        }
        capture.read_from_device_to_deque(&mut bytes).map_err(e)?;
        let samples: Vec<f32> = {
            let n = bytes.len() / 4 * 4;
            let raw: Vec<u8> = bytes.drain(..n).collect();
            raw.as_chunks::<4>().0.iter().map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]])).collect()
        };
        let mut mixer = mixer.lock();
        match source {
            Source::System => mixer.push_system(&samples),
            Source::Mic(_) => mixer.push_mic(&samples),
        }
    }
    let _ = client.stop_stream();
    Ok(())
}

/// Pulls exactly as much audio as the recording clock says has passed and gives it to the encoder.
/// Because it follows the clock, the sound is as long as the video even when devices go quiet.
fn drive(shared: &Shared, mixer: &Mutex<Mixer>, levels: &LevelsSink) {
    // Audio time zero is the first video frame, so picture and sound start together.
    let base = loop {
        if shared.stop.load(Ordering::Relaxed) {
            return;
        }
        if let Some(t) = shared.first_frame() {
            break t;
        }
        thread::sleep(Duration::from_millis(5));
    };

    let mut sent: u64 = 0;
    let mut peak = Levels::default();
    let mut last_report = Instant::now();
    loop {
        let stopping = shared.stop.load(Ordering::Acquire);

        if shared.flush_audio.swap(false, Ordering::AcqRel) {
            mixer.lock().clear();
        }

        let due = shared.now_ms().saturating_sub(base) * SAMPLE_RATE as u64 / 1000;
        while due > sent {
            let frames = (due - sent).min(4_800);
            let (pcm, lv) = mixer.lock().pull(frames as usize);
            peak = Levels { mic: peak.mic.max(lv.mic), system: peak.system.max(lv.system) };
            if let Some(encoder) = shared.encoder.lock().as_mut() {
                if let Err(e) = encoder.send_audio_buffer(&pcm, 0) {
                    eprintln!("sending audio failed: {e}");
                }
            }
            sent += frames;
        }

        if last_report.elapsed() >= Duration::from_millis(66) {
            levels(peak);
            peak = Levels::default();
            last_report = Instant::now();
        }
        if stopping {
            levels(Levels::default());
            return;
        }
        thread::sleep(Duration::from_millis(10));
    }
}
