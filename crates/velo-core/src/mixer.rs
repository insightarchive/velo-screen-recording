use std::collections::VecDeque;

pub const SAMPLE_RATE: u32 = 48_000;
pub const CHANNELS: usize = 2;

/// Peak level of each source over the last chunk, 0.0 to 1.0.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Levels {
    pub mic: f32,
    pub system: f32,
}

/// Mixes the microphone and system sound into one stereo 48 kHz stream.
///
/// Devices deliver audio in irregular bursts, and system loopback delivers nothing at all
/// during silence. So the mixer is *pulled* on a steady clock: every pull returns exactly the
/// requested number of frames, padding with silence when a source has nothing yet. That keeps
/// the audio exactly as long as the video.
#[derive(Debug)]
pub struct Mixer {
    system: VecDeque<f32>,
    mic: VecDeque<f32>,
    pub system_gain: f32,
    pub mic_gain: f32,
    /// Oldest audio beyond this is dropped so a stalled consumer cannot make audio lag.
    max_buffered_frames: usize,
}

impl Mixer {
    pub fn new(system_gain: f32, mic_gain: f32) -> Self {
        Self {
            system: VecDeque::new(),
            mic: VecDeque::new(),
            system_gain,
            mic_gain,
            max_buffered_frames: SAMPLE_RATE as usize / 2,
        }
    }

    /// Interleaved stereo f32 at 48 kHz.
    pub fn push_system(&mut self, samples: &[f32]) {
        Self::push(&mut self.system, samples, self.max_buffered_frames);
    }

    pub fn push_mic(&mut self, samples: &[f32]) {
        Self::push(&mut self.mic, samples, self.max_buffered_frames);
    }

    fn push(queue: &mut VecDeque<f32>, samples: &[f32], max_frames: usize) {
        queue.extend(samples);
        let max = max_frames * CHANNELS;
        if queue.len() > max {
            let excess = queue.len() - max;
            queue.drain(..excess - excess % CHANNELS);
        }
    }

    /// Produces `frames` stereo frames as little-endian signed 16-bit PCM, plus the levels.
    pub fn pull(&mut self, frames: usize) -> (Vec<u8>, Levels) {
        let mut pcm = Vec::with_capacity(frames * CHANNELS * 2);
        let (mut peak_mic, mut peak_sys) = (0.0f32, 0.0f32);
        for _ in 0..frames * CHANNELS {
            let s = self.system.pop_front().unwrap_or(0.0) * self.system_gain;
            let m = self.mic.pop_front().unwrap_or(0.0) * self.mic_gain;
            peak_sys = peak_sys.max(s.abs());
            peak_mic = peak_mic.max(m.abs());
            let mixed = soft_clip(s + m);
            pcm.extend_from_slice(&((mixed * i16::MAX as f32) as i16).to_le_bytes());
        }
        (pcm, Levels { mic: peak_mic.min(1.0), system: peak_sys.min(1.0) })
    }

    pub fn clear(&mut self) {
        self.system.clear();
        self.mic.clear();
    }

    #[cfg(test)]
    fn buffered(&self) -> (usize, usize) {
        (self.system.len(), self.mic.len())
    }
}

/// Transparent below 0.8, then smoothly compresses toward 1.0 so two loud sources never wrap or crackle.
fn soft_clip(x: f32) -> f32 {
    const KNEE: f32 = 0.8;
    let a = x.abs();
    if a <= KNEE {
        x
    } else {
        let over = (a - KNEE) / (1.0 - KNEE);
        x.signum() * (KNEE + (1.0 - KNEE) * over.tanh())
    }
}

/// Converts mono samples to interleaved stereo.
pub fn mono_to_stereo(mono: &[f32]) -> Vec<f32> {
    mono.iter().flat_map(|&s| [s, s]).collect()
}

/// Converts an interleaved buffer with any channel count to interleaved stereo.
pub fn to_stereo(samples: &[f32], channels: usize) -> Vec<f32> {
    match channels {
        0 => Vec::new(),
        1 => mono_to_stereo(samples),
        2 => samples.to_vec(),
        n => samples
            .chunks_exact(n)
            .flat_map(|f| [f[0], f[1]])
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn samples(pcm: &[u8]) -> Vec<i16> {
        pcm.as_chunks::<2>().0.iter().map(|b| i16::from_le_bytes([b[0], b[1]])).collect()
    }

    #[test]
    fn pull_always_returns_the_requested_length_and_pads_with_silence() {
        let mut m = Mixer::new(1.0, 1.0);
        let (pcm, levels) = m.pull(480);
        assert_eq!(pcm.len(), 480 * 2 * 2);
        assert!(samples(&pcm).iter().all(|&s| s == 0));
        assert_eq!(levels, Levels::default());
    }

    #[test]
    fn short_input_is_followed_by_silence_not_stale_data() {
        let mut m = Mixer::new(1.0, 1.0);
        m.push_system(&[0.5; 4]);
        let s = samples(&m.pull(4).0);
        assert!(s[..4].iter().all(|&v| v > 10_000) && s[4..].iter().all(|&v| v == 0));
    }

    #[test]
    fn sources_are_summed_and_levels_reported_per_source() {
        let mut m = Mixer::new(1.0, 1.0);
        m.push_system(&[0.2; 4]);
        m.push_mic(&[0.3; 4]);
        let (pcm, levels) = m.pull(2);
        let expected = (0.5 * i16::MAX as f32) as i16;
        assert!(samples(&pcm).iter().all(|&s| (s - expected).abs() < 3));
        assert!((levels.system - 0.2).abs() < 1e-6 && (levels.mic - 0.3).abs() < 1e-6);
    }

    #[test]
    fn loud_mix_is_limited_without_wrapping() {
        let mut m = Mixer::new(1.0, 1.0);
        m.push_system(&[1.0; 8]);
        m.push_mic(&[1.0; 8]);
        for s in samples(&m.pull(4).0) {
            assert!(s > 30_000, "no wraparound to negative, got {s}");
        }
    }

    #[test]
    fn gain_scales_each_source() {
        let mut m = Mixer::new(0.5, 0.0);
        m.push_system(&[0.8; 2]);
        m.push_mic(&[0.8; 2]);
        let (pcm, levels) = m.pull(1);
        let expected = (0.4 * i16::MAX as f32) as i16;
        assert!(samples(&pcm).iter().all(|&s| (s - expected).abs() < 3));
        assert_eq!(levels.mic, 0.0);
    }

    #[test]
    fn a_stalled_consumer_cannot_grow_the_buffer_forever() {
        let mut m = Mixer::new(1.0, 1.0);
        for _ in 0..100 {
            m.push_mic(&vec![0.1; 48_000 * 2]);
        }
        let (_, mic) = m.buffered();
        assert!(mic <= SAMPLE_RATE as usize / 2 * CHANNELS);
        assert_eq!(mic % CHANNELS, 0, "never split a stereo frame");
    }

    #[test]
    fn channel_conversion() {
        assert_eq!(mono_to_stereo(&[0.1, 0.2]), vec![0.1, 0.1, 0.2, 0.2]);
        assert_eq!(to_stereo(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 3), vec![1.0, 2.0, 4.0, 5.0]);
        assert_eq!(to_stereo(&[1.0, 2.0], 2), vec![1.0, 2.0]);
        assert!(to_stereo(&[1.0], 0).is_empty());
    }
}
