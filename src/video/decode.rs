//! H.264-in-MP4 decode session (openh264 + mp4 demux).

use std::io::Cursor;
use std::sync::Arc;
use std::time::Duration;

use openh264::decoder::{Decoder, DecoderConfig, Flush};
use openh264::formats::YUVSource;
use openh264::OpenH264API;

use super::avcc::Mp4BitstreamConverter;

/// Decoded poster / playback session for one MP4 byte buffer.
pub struct DecodeSession {
    mp4: mp4::Mp4Reader<Cursor<Arc<[u8]>>>,
    track_id: u32,
    sample_count: u32,
    timescale: u32,
    converter: Mp4BitstreamConverter,
    decoder: Decoder,
    pub width: u32,
    pub height: u32,
    /// Presentation time (seconds) for sample index `i` (0-based).
    sample_pts: Vec<f64>,
    pub duration: Duration,
    /// 1-based sample id last successfully fed to the decoder.
    next_sample: u32,
    buffer: Vec<u8>,
    rgba: Vec<u8>,
    /// Latest decoded frame (RGBA).
    pub frame: Option<(u32, u32, Vec<u8>)>,
}

impl DecodeSession {
    pub fn open(bytes: Arc<[u8]>) -> Result<Self, String> {
        if bytes.len() < 12 {
            return Err("Video too small".into());
        }
        // Reject obvious non-MP4 (WebM starts with EBML 0x1A45DFA3).
        if bytes.starts_with(&[0x1A, 0x45, 0xDF, 0xA3]) {
            return Err("WebM is not supported yet (H.264 MP4 only)".into());
        }

        let size = bytes.len() as u64;
        let mut mp4 = mp4::Mp4Reader::read_header(Cursor::new(bytes), size)
            .map_err(|e| format!("MP4 header: {e}"))?;

        let (track_id, width, height, sample_count, timescale, duration_secs) = {
            let track = mp4
                .tracks()
                .iter()
                .find(|(_, t)| matches!(t.track_type(), Ok(mp4::TrackType::Video)))
                .or_else(|| {
                    mp4.tracks()
                        .iter()
                        .find(|(_, t)| matches!(t.media_type(), Ok(mp4::MediaType::H264)))
                })
                .map(|(_, t)| t)
                .ok_or_else(|| "No video track in MP4".to_string())?;

            match track.media_type() {
                Ok(mp4::MediaType::H264) => {}
                Ok(other) => {
                    return Err(format!("Unsupported video codec in MP4: {other:?}"));
                }
                Err(e) => return Err(format!("Track media type: {e}")),
            }

            let track_id = track.track_id();
            let width = u32::from(track.width());
            let height = u32::from(track.height());
            let sample_count = track.sample_count();
            let timescale = track.timescale().max(1);
            let duration_secs = track.duration();
            (track_id, width, height, sample_count, timescale, duration_secs)
        };

        if sample_count == 0 || width == 0 || height == 0 {
            return Err("Empty video track".into());
        }

        let converter = {
            let track = mp4
                .tracks()
                .get(&track_id)
                .ok_or_else(|| "Missing track after probe".to_string())?;
            Mp4BitstreamConverter::for_mp4_track(track)?
        };

        let decoder_options = DecoderConfig::new().flush_after_decode(Flush::NoFlush);
        let decoder = Decoder::with_api_config(OpenH264API::from_source(), decoder_options)
            .map_err(|e| format!("OpenH264 init: {e}"))?;

        let mut sample_pts = Vec::with_capacity(sample_count as usize);
        for i in 1..=sample_count {
            if let Ok(Some(sample)) = mp4.read_sample(track_id, i) {
                sample_pts.push(sample.start_time as f64 / timescale as f64);
            } else {
                sample_pts.push(sample_pts.last().copied().unwrap_or(0.0));
            }
        }

        let mut session = Self {
            mp4,
            track_id,
            sample_count,
            timescale,
            converter,
            decoder,
            width,
            height,
            sample_pts,
            duration: duration_secs,
            next_sample: 1,
            buffer: Vec::new(),
            rgba: vec![0; (width as usize) * (height as usize) * 4],
            frame: None,
        };

        // Decode until we have a poster frame.
        session.decode_until_frame()?;
        if session.frame.is_none() {
            return Err("Could not decode a video frame".into());
        }
        Ok(session)
    }

    /// Advance decoding so a frame at or past `t` seconds is available.
    pub fn seek_playhead(&mut self, t: f64) -> Result<(), String> {
        let t = t.clamp(0.0, self.duration.as_secs_f64().max(0.0));
        // Find the first sample whose PTS is >= t (or the last sample).
        let mut target = self.sample_count;
        for (idx, pts) in self.sample_pts.iter().enumerate() {
            if *pts + f64::EPSILON >= t {
                target = (idx as u32) + 1;
                break;
            }
        }
        if target < self.next_sample {
            // Need to restart decoder to go backwards / loop.
            self.reset_decoder()?;
        }
        while self.next_sample <= target {
            if !self.feed_next_sample()? {
                break;
            }
        }
        Ok(())
    }

    fn reset_decoder(&mut self) -> Result<(), String> {
        let decoder_options = DecoderConfig::new().flush_after_decode(Flush::NoFlush);
        self.decoder = Decoder::with_api_config(OpenH264API::from_source(), decoder_options)
            .map_err(|e| format!("OpenH264 re-init: {e}"))?;
        // Rebuild converter (SPS inject state).
        let track = self
            .mp4
            .tracks()
            .get(&self.track_id)
            .ok_or_else(|| "Missing track".to_string())?;
        self.converter = Mp4BitstreamConverter::for_mp4_track(track)?;
        self.next_sample = 1;
        Ok(())
    }

    fn decode_until_frame(&mut self) -> Result<(), String> {
        while self.frame.is_none() && self.next_sample <= self.sample_count {
            self.feed_next_sample()?;
        }
        Ok(())
    }

    /// Feed one sample. Returns false when the stream is exhausted.
    fn feed_next_sample(&mut self) -> Result<bool, String> {
        if self.next_sample > self.sample_count {
            return Ok(false);
        }
        let sample_id = self.next_sample;
        self.next_sample += 1;

        let Some(sample) = self
            .mp4
            .read_sample(self.track_id, sample_id)
            .map_err(|e| format!("Read sample: {e}"))?
        else {
            return Ok(true);
        };

        self.converter
            .convert_packet(&sample.bytes, &mut self.buffer);
        if self.buffer.is_empty() {
            return Ok(true);
        }

        match self.decoder.decode(&self.buffer) {
            Ok(Some(yuv)) => {
                let (w, h) = yuv.dimensions();
                let need = w * h * 4;
                if self.rgba.len() != need {
                    self.rgba.resize(need, 0);
                    self.width = w as u32;
                    self.height = h as u32;
                }
                yuv.write_rgba8(&mut self.rgba);
                self.frame = Some((self.width, self.height, self.rgba.clone()));
            }
            Ok(None) => {}
            Err(e) => {
                // Soft-fail individual samples (B-frames / corruption).
                let _ = e;
            }
        }
        Ok(true)
    }

    pub fn ended(&self, t: f64) -> bool {
        t >= self.duration.as_secs_f64() && self.next_sample > self.sample_count
    }

    #[allow(dead_code)]
    pub fn timescale(&self) -> u32 {
        self.timescale
    }
}

#[cfg(all(test, feature = "video"))]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn decodes_sample_h264_mp4() {
        let path = PathBuf::from("/tmp/sleek-test.mp4");
        if !path.exists() {
            eprintln!("skip: missing {}", path.display());
            return;
        }
        let bytes: Arc<[u8]> = std::fs::read(&path).unwrap().into();
        let mut session = DecodeSession::open(bytes).expect("open");
        assert!(session.width > 0 && session.height > 0);
        assert!(session.frame.is_some());
        session.seek_playhead(0.5).expect("seek");
        assert!(session.frame.is_some());
    }
}
