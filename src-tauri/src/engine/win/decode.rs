//! Decodes an existing recording back into BGRA frames, for the export/render pipeline. Uses
//! Media Foundation's Source Reader, which handles any codec Windows itself can play (our own
//! H.264 mp4 included), so no bundled decoder is needed.

use std::path::Path;
use windows::core::HSTRING;
use windows::Win32::Media::MediaFoundation::{
    IMFAttributes, IMFMediaBuffer, IMFSample, IMFSourceReader, MFCreateAttributes, MFCreateMediaType,
    MFCreateSourceReaderFromURL, MFShutdown, MFStartup, MF_MT_FRAME_SIZE, MF_MT_MAJOR_TYPE, MF_MT_SUBTYPE,
    MF_SOURCE_READERF_ENDOFSTREAM, MF_SOURCE_READER_ENABLE_VIDEO_PROCESSING, MF_SOURCE_READER_FIRST_VIDEO_STREAM,
    MF_VERSION, MFMediaType_Video, MFVideoFormat_RGB32, MFSTARTUP_FULL,
};

pub struct Decoder {
    reader: IMFSourceReader,
    width: u32,
    height: u32,
    /// The most recently decoded frame: its timestamp (100ns units) and BGRA8 bytes.
    current: Option<(i64, Vec<u8>)>,
    eof: bool,
}

impl Decoder {
    pub fn open(path: &Path) -> Result<Self, String> {
        unsafe { MFStartup(MF_VERSION, MFSTARTUP_FULL) }.map_err(|e| e.to_string())?;

        // Without this, the reader will only hand back the stream's native (compressed) type and
        // refuses to convert to RGB32 for us.
        let mut attrs: Option<IMFAttributes> = None;
        unsafe { MFCreateAttributes(&mut attrs, 1) }.map_err(|e| e.to_string())?;
        let attrs = attrs.ok_or("Media Foundation did not return an attribute store.")?;
        unsafe { attrs.SetUINT32(&MF_SOURCE_READER_ENABLE_VIDEO_PROCESSING, 1) }.map_err(|e| e.to_string())?;

        let wide = HSTRING::from(path.as_os_str());
        let reader: IMFSourceReader =
            unsafe { MFCreateSourceReaderFromURL(&wide, &attrs) }.map_err(|e| e.to_string())?;

        let want = unsafe { MFCreateMediaType() }.map_err(|e| e.to_string())?;
        unsafe {
            want.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video).map_err(|e| e.to_string())?;
            want.SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_RGB32).map_err(|e| e.to_string())?;
            reader
                .SetCurrentMediaType(MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32, None, &want)
                .map_err(|e| e.to_string())?;
        }

        let negotiated = unsafe { reader.GetCurrentMediaType(MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32) }
            .map_err(|e| e.to_string())?;
        let packed = unsafe { negotiated.GetUINT64(&MF_MT_FRAME_SIZE) }.map_err(|e| e.to_string())?;
        let width = (packed >> 32) as u32;
        let height = (packed & 0xFFFF_FFFF) as u32;
        if width == 0 || height == 0 {
            return Err("Could not read the recording's frame size.".into());
        }

        Ok(Self { reader, width, height, current: None, eof: false })
    }

    pub const fn width(&self) -> u32 {
        self.width
    }

    pub const fn height(&self) -> u32 {
        self.height
    }

    /// Decodes forward (never backward) until the current frame's timestamp has reached
    /// `target_100ns`, then returns its BGRA8 bytes. Once the source ends, keeps returning the
    /// last frame, so trailing output frames just hold on the final picture.
    pub fn frame_at(&mut self, target_100ns: i64) -> Result<&[u8], String> {
        while !self.eof && self.current.as_ref().map(|(t, _)| *t).unwrap_or(i64::MIN) < target_100ns {
            self.advance()?;
        }
        self.current.as_ref().map(|(_, buf)| buf.as_slice()).ok_or_else(|| "No frames in the recording.".to_string())
    }

    fn advance(&mut self) -> Result<(), String> {
        let mut flags = 0u32;
        let mut timestamp: i64 = 0;
        let mut sample: Option<IMFSample> = None;
        unsafe {
            self.reader.ReadSample(
                MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32,
                0,
                None,
                Some(&mut flags),
                Some(&mut timestamp),
                Some(&mut sample),
            )
        }
        .map_err(|e| e.to_string())?;

        if (flags & MF_SOURCE_READERF_ENDOFSTREAM.0 as u32) != 0 {
            self.eof = true;
        }
        let Some(sample) = sample else {
            self.eof = true;
            return Ok(());
        };

        let buffer: IMFMediaBuffer = unsafe { sample.ConvertToContiguousBuffer() }.map_err(|e| e.to_string())?;
        let mut data: *mut u8 = std::ptr::null_mut();
        let mut len = 0u32;
        unsafe { buffer.Lock(&mut data, None, Some(&mut len)) }.map_err(|e| e.to_string())?;
        let owned = unsafe { std::slice::from_raw_parts(data, len as usize) }.to_vec();
        let _ = unsafe { buffer.Unlock() };

        self.current = Some((timestamp, owned));
        Ok(())
    }
}

impl Drop for Decoder {
    fn drop(&mut self) {
        let _ = unsafe { MFShutdown() };
    }
}
