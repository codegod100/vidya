//! AVCC (length-prefixed) → Annex B converter for H.264-in-MP4.
//!
//! Adapted from the `openh264` crate's `examples/mp4` helper (BSD-2-Clause).

use mp4::Mp4Track;

/// Network abstraction layer type for an H.264 NAL unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NalType {
    Unspecified = 0,
    Slice = 1,
    IdrSlice = 5,
    Sps = 7,
    Pps = 8,
    Other,
}

impl From<u8> for NalType {
    fn from(value: u8) -> Self {
        match value & 0x1F {
            0 => Self::Unspecified,
            1 => Self::Slice,
            5 => Self::IdrSlice,
            7 => Self::Sps,
            8 => Self::Pps,
            _ => Self::Other,
        }
    }
}

struct NalUnit<'a> {
    nal_type: NalType,
    bytes: &'a [u8],
}

impl<'a> NalUnit<'a> {
    fn from_stream(mut stream: &'a [u8], length_size: u8) -> Option<(Self, &'a [u8])> {
        if stream.len() < length_size as usize {
            return None;
        }
        let mut nal_size = 0u32;
        for _ in 0..length_size {
            nal_size = (nal_size << 8) | u32::from(stream[0]);
            stream = &stream[1..];
        }
        if nal_size == 0 || stream.len() < nal_size as usize {
            return None;
        }
        let packet = &stream[..nal_size as usize];
        let nal_type = NalType::from(packet[0]);
        let unit = NalUnit {
            nal_type,
            bytes: packet,
        };
        stream = &stream[nal_size as usize..];
        Some((unit, stream))
    }
}

/// Convert MP4/AVCC length-prefixed NAL units to Annex B for OpenH264.
///
/// Also injects SPS/PPS from the `avcC` box when an IDR arrives without them.
pub struct Mp4BitstreamConverter {
    length_size: u8,
    sps: Vec<Vec<u8>>,
    pps: Vec<Vec<u8>>,
    new_idr: bool,
    sps_seen: bool,
    pps_seen: bool,
}

impl Mp4BitstreamConverter {
    pub fn for_mp4_track(track: &Mp4Track) -> Result<Self, String> {
        let avcc_config = &track
            .trak
            .mdia
            .minf
            .stbl
            .stsd
            .avc1
            .as_ref()
            .ok_or_else(|| "Track does not contain AVC1/avcC config".to_string())?
            .avcc;

        Ok(Self {
            length_size: avcc_config.length_size_minus_one + 1,
            sps: avcc_config
                .sequence_parameter_sets
                .iter()
                .cloned()
                .map(|v| v.bytes)
                .collect(),
            pps: avcc_config
                .picture_parameter_sets
                .iter()
                .cloned()
                .map(|v| v.bytes)
                .collect(),
            new_idr: true,
            sps_seen: false,
            pps_seen: false,
        })
    }

    pub fn convert_packet(&mut self, packet: &[u8], out: &mut Vec<u8>) {
        let mut stream = packet;
        out.clear();

        while !stream.is_empty() {
            let Some((unit, remaining_stream)) = NalUnit::from_stream(stream, self.length_size)
            else {
                break;
            };
            stream = remaining_stream;

            match unit.nal_type {
                NalType::Sps => self.sps_seen = true,
                NalType::Pps => self.pps_seen = true,
                NalType::IdrSlice => {
                    if !self.new_idr && unit.bytes.len() > 1 && unit.bytes[1] & 0x80 != 0 {
                        self.new_idr = true;
                    }
                    if self.new_idr && !self.sps_seen && !self.pps_seen {
                        self.new_idr = false;
                        for sps in &self.sps {
                            out.extend([0, 0, 1]);
                            out.extend(sps);
                        }
                        for pps in &self.pps {
                            out.extend([0, 0, 1]);
                            out.extend(pps);
                        }
                    }
                    if self.new_idr && self.sps_seen && !self.pps_seen {
                        for pps in &self.pps {
                            out.extend([0, 0, 1]);
                            out.extend(pps);
                        }
                    }
                }
                _ => {}
            }

            out.extend([0, 0, 1]);
            out.extend(unit.bytes);

            if !self.new_idr && unit.nal_type == NalType::Slice {
                self.new_idr = true;
                self.sps_seen = false;
                self.pps_seen = false;
            }
        }
    }
}
