//! A JPEG thumbnail for a recorded video, using the Windows Shell's own thumbnail provider
//! (the same one Explorer uses), so no video decoder is needed here.

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use std::ffi::c_void;
use windows::core::{Interface, HSTRING};
use windows::Win32::Foundation::SIZE;
use windows::Win32::Graphics::Gdi::{
    DeleteObject, GetDC, GetDIBits, GetObjectW, ReleaseDC, BITMAP, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
    DIB_RGB_COLORS,
};
use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED};
use windows::Win32::UI::Shell::{IShellItem, IShellItemImageFactory, SHCreateItemFromParsingName, SIIGBF_RESIZETOFIT};
use windows_capture::encoder::{ImageEncoder, ImageEncoderPixelFormat, ImageFormat};

/// Target edge length; the shell returns an image that fits inside it, aspect preserved.
const SIZE_PX: i32 = 480;

pub fn thumbnail(path: &str) -> Result<String, String> {
    // Balanced by `CoUninitialize` below when this call is the one that initialized the apartment.
    let co = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    let result = extract(path);
    if co.is_ok() {
        unsafe { CoUninitialize() };
    }
    result
}

fn extract(path: &str) -> Result<String, String> {
    let wide = HSTRING::from(path);
    let item: IShellItem = unsafe { SHCreateItemFromParsingName(&wide, None) }.map_err(|e| e.to_string())?;
    let factory: IShellItemImageFactory = item.cast().map_err(|e| e.to_string())?;
    let hbitmap = unsafe { factory.GetImage(SIZE { cx: SIZE_PX, cy: SIZE_PX }, SIIGBF_RESIZETOFIT) }.map_err(|e| e.to_string())?;

    let result = (|| {
        let mut bmp = BITMAP::default();
        let wrote = unsafe {
            GetObjectW(hbitmap.into(), std::mem::size_of::<BITMAP>() as i32, Some(&mut bmp as *mut _ as *mut c_void))
        };
        if wrote == 0 {
            return Err("Could not read the thumbnail bitmap.".to_string());
        }
        let (width, height) = (bmp.bmWidth, bmp.bmHeight);

        let hdc = unsafe { GetDC(None) };
        let mut info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                // Negative height requests a top-down DIB, matching how we feed it to the JPEG encoder.
                biHeight: -height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut buffer = vec![0u8; (width * height * 4) as usize];
        let read = unsafe {
            GetDIBits(hdc, hbitmap, 0, height as u32, Some(buffer.as_mut_ptr() as *mut c_void), &mut info, DIB_RGB_COLORS)
        };
        unsafe { ReleaseDC(None, hdc) };
        if read == 0 {
            return Err("Could not read the thumbnail pixels.".to_string());
        }

        // Shell thumbnail bitmaps often report a zero alpha channel; a flat thumbnail has no real
        // transparency, so force it opaque rather than rendering as a blank image.
        for px in buffer.chunks_exact_mut(4) {
            px[3] = 0xFF;
        }

        let encoder = ImageEncoder::new(ImageFormat::Jpeg, ImageEncoderPixelFormat::Bgra8).map_err(|e| e.to_string())?;
        let jpeg = encoder.encode(&buffer, width as u32, height as u32).map_err(|e| e.to_string())?;
        Ok(format!("data:image/jpeg;base64,{}", BASE64.encode(jpeg)))
    })();

    let _ = unsafe { DeleteObject(hbitmap.into()) };
    result
}
