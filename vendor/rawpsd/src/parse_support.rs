use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::cursor::SliceCursor;
use crate::layer_types::{MaskInfo, PsdMetadata};

pub(crate) fn read_u8(cursor: &mut SliceCursor) -> Result<u8, String> {
    let mut buf = [0; 1];
    cursor.read_exact(&mut buf).map_err(|x| x.to_string())?;
    Ok(buf[0])
}

pub(crate) fn read_u16(cursor: &mut SliceCursor) -> Result<u16, String> {
    let mut buf = [0; 2];
    cursor.read_exact(&mut buf).map_err(|x| x.to_string())?;
    Ok(u16::from_be_bytes(buf))
}

pub(crate) fn read_u32(cursor: &mut SliceCursor) -> Result<u32, String> {
    let mut buf = [0; 4];
    cursor.read_exact(&mut buf).map_err(|x| x.to_string())?;
    Ok(u32::from_be_bytes(buf))
}

pub(crate) fn read_b4(cursor: &mut SliceCursor) -> Result<[u8; 4], String> {
    let mut buf = [0; 4];
    cursor.read_exact(&mut buf).map_err(|x| x.to_string())?;
    Ok(buf)
}

pub(crate) fn read_i32(cursor: &mut SliceCursor) -> Result<i32, String> {
    let mut buf = [0; 4];
    cursor.read_exact(&mut buf).map_err(|x| x.to_string())?;
    Ok(i32::from_be_bytes(buf))
}

pub(crate) fn read_f64(cursor: &mut SliceCursor) -> Result<f64, String> {
    let mut buf = [0; 8];
    cursor.read_exact(&mut buf).map_err(|x| x.to_string())?;
    Ok(f64::from_be_bytes(buf))
}

pub(crate) fn seek_to(cursor: &mut SliceCursor, pos: u64) -> Result<(), String> {
    if pos > cursor.buf.len() as u64 {
        return Err("Unexpeted end of stream".to_string());
    }
    cursor.set_position(pos);
    Ok(())
}

fn set_mask_bounds(mask_info: &mut MaskInfo, top: i32, left: i32, bottom: i32, right: i32) {
    mask_info.x = left;
    mask_info.y = top;

    if right >= left {
        mask_info.w = (right - left) as u32;
    }

    if bottom >= top {
        mask_info.h = (bottom - top) as u32;
    }
}

pub(crate) fn read_mask_info(cursor: &mut SliceCursor) -> Result<MaskInfo, String> {
    let maskdat_len = read_u32(cursor)? as u64;
    let maskdat_start = cursor.position();
    let maskdat_end = maskdat_start
        .checked_add(maskdat_len)
        .ok_or("Mask data length overflow".to_string())?;

    if maskdat_end > cursor.buf.len() as u64 {
        return Err("Unexpeted end of stream".to_string());
    }

    let mut mask_info = MaskInfo::default();
    if maskdat_len == 0 {
        return Ok(mask_info);
    }

    let mut remaining = maskdat_len;

    if remaining < 16 {
        seek_to(cursor, maskdat_end)?;
        return Ok(mask_info);
    }

    let top = read_i32(cursor)?;
    let left = read_i32(cursor)?;
    let bottom = read_i32(cursor)?;
    let right = read_i32(cursor)?;
    remaining -= 16;
    set_mask_bounds(&mut mask_info, top, left, bottom, right);

    if remaining < 1 {
        seek_to(cursor, maskdat_end)?;
        return Ok(mask_info);
    }

    mask_info.default_color = read_u8(cursor)?;
    remaining -= 1;

    if remaining < 1 {
        seek_to(cursor, maskdat_end)?;
        return Ok(mask_info);
    }

    let mflags = read_u8(cursor)?;
    remaining -= 1;
    mask_info.relative = (mflags & 1) != 0;
    mask_info.disabled = (mflags & 2) != 0;
    mask_info.invert = (mflags & 4) != 0;

    if (mflags & 16) != 0 {
        if remaining < 1 {
            seek_to(cursor, maskdat_end)?;
            return Ok(mask_info);
        }

        let mask_params = read_u8(cursor)?;
        remaining -= 1;

        if (mask_params & 1) != 0 {
            if remaining < 1 {
                seek_to(cursor, maskdat_end)?;
                return Ok(mask_info);
            }
            read_u8(cursor)?;
            remaining -= 1;
        }

        if (mask_params & 2) != 0 {
            if remaining < 8 {
                seek_to(cursor, maskdat_end)?;
                return Ok(mask_info);
            }
            read_f64(cursor)?;
            remaining -= 8;
        }

        if (mask_params & 4) != 0 {
            if remaining < 1 {
                seek_to(cursor, maskdat_end)?;
                return Ok(mask_info);
            }
            read_u8(cursor)?;
            remaining -= 1;
        }

        if (mask_params & 8) != 0 {
            if remaining < 8 {
                seek_to(cursor, maskdat_end)?;
                return Ok(mask_info);
            }
            read_f64(cursor)?;
            remaining -= 8;
        }
    }

    if maskdat_len == 20 {
        seek_to(cursor, maskdat_end)?;
        return Ok(mask_info);
    }

    if remaining < 1 {
        seek_to(cursor, maskdat_end)?;
        return Ok(mask_info);
    }
    read_u8(cursor)?;
    remaining -= 1;

    if remaining < 1 {
        seek_to(cursor, maskdat_end)?;
        return Ok(mask_info);
    }
    read_u8(cursor)?;
    remaining -= 1;

    if remaining < 16 {
        seek_to(cursor, maskdat_end)?;
        return Ok(mask_info);
    }
    let _real_top = read_i32(cursor)?;
    let _real_left = read_i32(cursor)?;
    let _real_bottom = read_i32(cursor)?;
    let _real_right = read_i32(cursor)?;

    seek_to(cursor, maskdat_end)?;
    Ok(mask_info)
}

/// Parses just the frontmost metadata at the start of a PSD file.
///
/// You will need to use both this and [crate::parse_layer_records].
pub fn parse_psd_metadata(data: &[u8]) -> Result<PsdMetadata, String> {
    let mut cursor = SliceCursor::new(data);

    let signature = read_b4(&mut cursor)?;
    if signature != [0x38, 0x42, 0x50, 0x53] {
        return Err("Invalid PSD signature".to_string());
    }

    let version = read_u16(&mut cursor)?;
    if version != 1 {
        return Err("Unsupported PSD version".to_string());
    }

    cursor.set_position(cursor.position() + 6);

    let channel_count = read_u16(&mut cursor)?;
    let height = read_u32(&mut cursor)?;
    let width = read_u32(&mut cursor)?;
    let depth = read_u16(&mut cursor)?;
    let color_mode = read_u16(&mut cursor)?;

    Ok(PsdMetadata {
        width,
        height,
        channel_count,
        depth,
        color_mode,
    })
}

/// Decompress a packbits image data buffer into a vec, appending to the vec.
pub fn append_img_data(cursor: &[u8], output: &mut Vec<u8>, size: u64, h: u64) -> Result<usize, String> {
    let mut cursor = SliceCursor::new(cursor);
    let mode = read_u16(&mut cursor)?;
    if mode == 0 {
        cursor.take(size).read_to_end(output).map_err(|x| x.to_string())?;
    } else if mode == 1 {
        let mut c2 = cursor.clone();
        c2.set_position(c2.position() + h * 2);
        for _ in 0..h {
            let len = read_u16(&mut cursor)?;
            let start = c2.position();
            while c2.position() < start + len as u64 {
                let n = read_u8(&mut c2)? as i8;
                if n >= 0 {
                    c2.take(n as u64 + 1)
                        .read_to_end(output)
                        .map_err(|x| x.to_string())?;
                } else if n != -128 {
                    output.extend(core::iter::repeat_n(read_u8(&mut c2)?, (1 - n as i64) as usize));
                }
            }
        }
        cursor.set_position(c2.position());
    } else {
        return Err("unsupported compression format".to_string());
    }
    Ok(cursor.position() as usize)
}

/// Decompress a packbits image data buffer into a slice, writing into the slice in-place. `stride` can be used to control how far apart to write each byte.
pub fn copy_img_data(
    cursor: &[u8],
    output: &mut [u8],
    stride: usize,
    size: u64,
    h: u64,
) -> Result<usize, String> {
    let mut cursor = SliceCursor::new(cursor);
    let pos = cursor.position();
    let mode = read_u16(&mut cursor)?;
    if mode == 0 {
        for i in 0..size as usize - 2 {
            output[i * stride] = read_u8(&mut cursor)?;
        }
    } else if mode == 1 {
        let mut c2 = cursor.clone();
        c2.set_position(c2.position() + h * 2);
        let mut i = 0;
        let mut j = 2;
        for _ in 0..h {
            let len = read_u16(&mut cursor)?;
            j += 2;
            let start = c2.position();
            while c2.position() - start < len as u64 {
                let n = read_u8(&mut c2)? as i8;
                j += 1;
                if n >= 0 {
                    for _ in 0..n as u64 + 1 {
                        let c = read_u8(&mut c2)?;
                        if i * stride < output.len() {
                            output[i * stride] = c;
                        }
                        i += 1;
                        j += 1;
                    }
                } else if n != -128 {
                    let c = read_u8(&mut c2)?;
                    for _ in 0..1 - n as i64 {
                        if i * stride < output.len() {
                            output[i * stride] = c;
                        }
                        i += 1;
                    }
                    j += 1;
                }
            }
            c2.set_position(start + len as u64);
        }
        if j != size {
            return Err("Desynchronized while reading image data".to_string());
        }
    } else {
        return Err(format!("unsupported compression format {} at 0x{:X}", mode, pos));
    }
    Ok(size as usize)
}
