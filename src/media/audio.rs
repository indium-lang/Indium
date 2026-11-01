use sdl2::audio::{AudioCallback, AudioDevice, AudioSpecDesired};
use std::{
    collections::HashMap,
    fs::File,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use symphonia::core::{
    audio::SampleBuffer, codecs::DecoderOptions, errors::Error, formats::FormatOptions,
    io::MediaSourceStream, meta::MetadataOptions, probe::Hint,
};

pub struct Clip {
    pub samples: Vec<f32>,
    pub rate: u32,
    pub channels: usize,
}
struct Voice {
    id: u64,
    clip: Arc<Clip>,
    position: f64,
    volume: f32,
    looping: bool,
    paused: bool,
}
#[derive(Default)]
struct Mixer {
    voices: Vec<Voice>,
}
struct Callback {
    mixer: Arc<Mutex<Mixer>>,
    rate: u32,
}
impl AudioCallback for Callback {
    type Channel = f32;
    fn callback(&mut self, out: &mut [f32]) {
        out.fill(0.0);
        let Ok(mut mixer) = self.mixer.lock() else {
            return;
        };
        for voice in &mut mixer.voices {
            if voice.paused {
                continue;
            }
            let frames = voice.clip.samples.len() / voice.clip.channels;
            if frames == 0 {
                continue;
            }
            let step = voice.clip.rate as f64 / self.rate as f64;
            for frame in out.chunks_exact_mut(2) {
                if voice.position >= frames as f64 {
                    if voice.looping {
                        voice.position %= frames as f64;
                    } else {
                        break;
                    }
                }
                let index = voice.position as usize;
                let next = (index + 1).min(frames - 1);
                let fraction = (voice.position - index as f64) as f32;
                for (channel, sample) in frame.iter_mut().enumerate() {
                    let ch = channel.min(voice.clip.channels - 1);
                    let a = voice.clip.samples[index * voice.clip.channels + ch];
                    let b = voice.clip.samples[next * voice.clip.channels + ch];
                    *sample += (a + (b - a) * fraction) * voice.volume;
                }
                voice.position += step;
            }
        }
        mixer
            .voices
            .retain(|v| v.looping || v.position < (v.clip.samples.len() / v.clip.channels) as f64);
        for sample in out {
            *sample = sample.clamp(-1.0, 1.0);
        }
    }
}
pub struct AudioEngine {
    device: AudioDevice<Callback>,
    mixer: Arc<Mutex<Mixer>>,
    cache: HashMap<PathBuf, Arc<Clip>>,
    next_id: u64,
}
impl AudioEngine {
    pub fn new(sdl: &sdl2::Sdl) -> Result<Self, String> {
        let audio = sdl.audio()?;
        let mixer = Arc::new(Mutex::new(Mixer::default()));
        let shared = mixer.clone();
        let device = audio.open_playback(
            None,
            &AudioSpecDesired {
                freq: Some(48000),
                channels: Some(2),
                samples: Some(512),
            },
            move |spec| Callback {
                mixer: shared,
                rate: spec.freq as u32,
            },
        )?;
        device.resume();
        Ok(Self {
            device,
            mixer,
            cache: HashMap::new(),
            next_id: 0,
        })
    }
    pub fn play(&mut self, path: &Path, volume: f32, looping: bool) -> Result<u64, String> {
        let path = path
            .canonicalize()
            .map_err(|e| format!("Cannot open sound '{}': {e}", path.display()))?;
        let clip = if let Some(c) = self.cache.get(&path) {
            c.clone()
        } else {
            let clip = Arc::new(decode(&path)?);
            self.cache.insert(path, clip.clone());
            clip
        };
        let mut mixer = self
            .mixer
            .lock()
            .map_err(|_| "Audio mixer is unavailable")?;
        if mixer.voices.len() >= 128 {
            return Err("At most 128 sounds can play at once".into());
        }
        self.next_id = self
            .next_id
            .checked_add(1)
            .ok_or("Audio handle limit reached")?;
        mixer.voices.push(Voice {
            id: self.next_id,
            clip,
            position: 0.0,
            volume,
            looping,
            paused: false,
        });
        Ok(self.next_id)
    }
    pub fn control(&mut self, action: &str, id: i64, volume: Option<f32>) -> Result<bool, String> {
        if id < 1 {
            return Err("Audio handle must be a positive integer returned by audio.play".into());
        }
        let mut mixer = self
            .mixer
            .lock()
            .map_err(|_| "Audio mixer is unavailable")?;
        let Some(index) = mixer.voices.iter().position(|v| v.id == id as u64) else {
            return Ok(false);
        };
        match action {
            "pause" => mixer.voices[index].paused = true,
            "resume" => mixer.voices[index].paused = false,
            "stop" => {
                mixer.voices.remove(index);
            }
            "volume" => mixer.voices[index].volume = volume.ok_or("Missing volume")?,
            "playing" => return Ok(!mixer.voices[index].paused),
            _ => return Err(format!("Unknown audio action '{action}'")),
        };
        Ok(true)
    }
}
impl Drop for AudioEngine {
    fn drop(&mut self) {
        self.device.pause();
    }
}

pub fn decode(path: &Path) -> Result<Clip, String> {
    let file = File::open(path).map_err(|e| e.to_string())?;
    let stream = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
        hint.with_extension(ext);
    }
    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            stream,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .map_err(|e| format!("Cannot decode sound: {e}"))?;
    let mut format = probed.format;
    let track = format.default_track().ok_or("Sound has no audio track")?;
    let id = track.id;
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| e.to_string())?;
    let mut samples = Vec::new();
    let mut rate = 0;
    let mut channels = 0;
    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(Error::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(format!("Sound read failed: {e}")),
        };
        if packet.track_id() != id {
            continue;
        }
        let decoded = decoder
            .decode(&packet)
            .map_err(|e| format!("Sound decode failed: {e}"))?;
        let spec = *decoded.spec();
        let count = spec.channels.count();
        if count == 0 || count > 2 {
            return Err("Only mono and stereo audio are supported in this pre-alpha".into());
        }
        if rate != 0 && (rate != spec.rate || channels != count) {
            return Err("Audio format changes within a stream are not supported".into());
        }
        rate = spec.rate;
        channels = count;
        if rate == 0 {
            return Err("Invalid audio sample rate".into());
        }
        let mut buffer = SampleBuffer::<f32>::new(decoded.capacity() as u64, spec);
        buffer.copy_interleaved_ref(decoded);
        if samples.len() + buffer.samples().len() > 64 * 1024 * 1024 {
            return Err("Decoded sound exceeds 256 MiB; streaming is not implemented yet".into());
        }
        samples.extend_from_slice(buffer.samples());
    }
    if samples.is_empty() {
        return Err("Sound contains no decoded samples".into());
    }
    if samples.iter().any(|v| !v.is_finite()) {
        return Err("Sound contains non-finite samples".into());
    }
    Ok(Clip {
        samples,
        rate,
        channels,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ogg_and_wav_decode_real_fixtures() {
        for name in ["tone.wav", "tone.ogg"] {
            let clip = decode(
                &Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("examples/assets")
                    .join(name),
            )
            .unwrap();
            assert_eq!(clip.rate, 44100);
            assert_eq!(clip.channels, 1);
            let seconds = clip.samples.len() as f64 / clip.rate as f64;
            assert!((0.48..0.55).contains(&seconds), "{name}: {seconds}");
            let energy =
                clip.samples.iter().map(|s| s * s).sum::<f32>() / clip.samples.len() as f32;
            assert!((0.005..0.015).contains(&energy), "{name}: {energy}");
        }
    }
    #[test]
    fn concurrent_voices_mix_pause_loop_and_finish() {
        let clip = Arc::new(Clip {
            samples: vec![0.75; 8],
            rate: 48000,
            channels: 2,
        });
        let mixer = Arc::new(Mutex::new(Mixer {
            voices: vec![
                Voice {
                    id: 1,
                    clip: clip.clone(),
                    position: 0.0,
                    volume: 1.0,
                    looping: false,
                    paused: false,
                },
                Voice {
                    id: 2,
                    clip: clip.clone(),
                    position: 0.0,
                    volume: 1.0,
                    looping: true,
                    paused: false,
                },
                Voice {
                    id: 3,
                    clip,
                    position: 0.0,
                    volume: 1.0,
                    looping: true,
                    paused: true,
                },
            ],
        }));
        let mut callback = Callback {
            mixer: mixer.clone(),
            rate: 48000,
        };
        let mut output = [0.0; 8];
        callback.callback(&mut output);
        assert_eq!(output, [1.0; 8]);
        assert_eq!(mixer.lock().unwrap().voices.len(), 2);
        callback.callback(&mut output);
        assert_eq!(output, [0.75; 8]);
        assert_eq!(mixer.lock().unwrap().voices[1].position, 0.0);
    }
}
