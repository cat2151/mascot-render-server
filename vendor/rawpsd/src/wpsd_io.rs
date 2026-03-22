use std::io::Cursor;
use std::io::Read;

use crate::wpsd_types::{MaskInfo, PsdMetadata};

pub(crate) fn read_u8(cursor: &mut Cursor<&[u8]>) -> u8 {
    let mut buf = [0; 1];
    cursor.read_exact(&mut buf).expect("Failed to read u8");
    buf[0]
}

pub(crate) fn read_u16(cursor: &mut Cursor<&[u8]>) -> u16 {
    let mut buf = [0; 2];
    cursor.read_exact(&mut buf).expect("Failed to read u16");
    u16::from_be_bytes(buf)
}

pub(crate) fn read_u32(cursor: &mut Cursor<&[u8]>) -> u32 {
    let mut buf = [0; 4];
    cursor.read_exact(&mut buf).expect("Failed to read u32");
    u32::from_be_bytes(buf)
}

pub(crate) fn read_i32(cursor: &mut Cursor<&[u8]>) -> i32 {
    let mut buf = [0; 4];
    cursor.read_exact(&mut buf).expect("Failed to read i32");
    i32::from_be_bytes(buf)
}

pub(crate) fn read_f32(cursor: &mut Cursor<&[u8]>) -> f32 {
    let mut buf = [0; 4];
    cursor.read_exact(&mut buf).expect("Failed to read f32");
    f32::from_be_bytes(buf)
}

pub(crate) fn read_f64(cursor: &mut Cursor<&[u8]>) -> f64 {
    let mut buf = [0; 8];
    cursor.read_exact(&mut buf).expect("Failed to read f64");
    f64::from_be_bytes(buf)
}

pub fn parse_psd_metadata(data: &[u8]) -> PsdMetadata {
    let mut cursor = Cursor::new(data);

    let mut signature = [0; 4];
    cursor
        .read_exact(&mut signature)
        .expect("Failed to read PSD signature");
    if signature != [0x38, 0x42, 0x50, 0x53] {
        panic!("Invalid PSD signature");
    }

    let version = read_u16(&mut cursor);
    if version != 1 {
        panic!("Unsupported PSD version");
    }

    cursor.set_position(cursor.position() + 6);

    let channel_count = read_u16(&mut cursor);
    let height = read_u32(&mut cursor);
    let width = read_u32(&mut cursor);
    let depth = read_u16(&mut cursor);
    let color_mode = read_u16(&mut cursor);

    PsdMetadata {
        width,
        height,
        channel_count,
        depth,
        color_mode,
    }
}

pub fn append_img_data(cursor: &mut Cursor<&[u8]>, output: &mut Vec<u8>, size: u64, h: u64) {
    let mode = read_u16(cursor);
    if mode == 0 {
        cursor.take(size).read_to_end(output).unwrap();
    } else if mode == 1 {
        let mut c2 = cursor.clone();
        c2.set_position(c2.position() + h * 2);
        for _ in 0..h {
            let len = read_u16(cursor);
            let start = c2.position();
            while c2.position() < start + len as u64 {
                let n = read_u8(&mut c2) as i8;
                if n >= 0 {
                    (&mut c2).take(n as u64 + 1).read_to_end(output).unwrap();
                } else if n != -128 {
                    output.extend(std::iter::repeat_n(read_u8(&mut c2), (1 - n as i64) as usize));
                }
            }
        }
        cursor.set_position(c2.position());
    } else {
        panic!("unsupported compression format");
    }
}

pub fn copy_img_data(
    cursor: &mut Cursor<&[u8]>,
    output: &mut [u8],
    stride: usize,
    size: u64,
    h: u64,
) {
    let pos = cursor.position();
    let mode = read_u16(cursor);
    if mode == 0 {
        for i in 0..size as usize - 2 {
            output[i * stride] = read_u8(cursor);
        }
    } else if mode == 1 {
        let mut c2 = cursor.clone();
        c2.set_position(c2.position() + h * 2);
        let mut i = 0;
        let mut j = 2;
        for _ in 0..h {
            let len = read_u16(cursor);
            j += 2;
            let start = c2.position();
            while c2.position() - start < len as u64 {
                let n = read_u8(&mut c2) as i8;
                j += 1;
                if n >= 0 {
                    for _ in 0..n as u64 + 1 {
                        let c = read_u8(&mut c2);
                        if i * stride < output.len() {
                            output[i * stride] = c;
                        }
                        i += 1;
                        j += 1;
                    }
                } else if n != -128 {
                    let c = read_u8(&mut c2);
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
        assert!(j == size, "{} {}", j, size);
    } else {
        panic!("unsupported compression format {} at 0x{:X}", mode, pos);
    }
    cursor.set_position(pos + size);
}

pub(crate) fn read_mask_info(cursor: &mut Cursor<&[u8]>) -> MaskInfo {
    let maskdat_len = read_u32(cursor) as u64;
    let maskdat_start = cursor.position();
    let mtop = read_i32(cursor);
    let mleft = read_i32(cursor);
    let mbottom = read_i32(cursor);
    let mright = read_i32(cursor);
    let mut mask_info = MaskInfo::default();
    mask_info.x = mleft;
    mask_info.y = mtop;
    mask_info.w = (mright - mleft) as u32;
    mask_info.h = (mbottom - mtop) as u32;
    mask_info.default_color = read_u8(cursor);
    let mflags = read_u8(cursor);
    mask_info.relative = (mflags & 1) != 0;
    mask_info.disabled = (mflags & 2) != 0;
    mask_info.invert = (mflags & 4) != 0;
    cursor.set_position(maskdat_start + maskdat_len);
    mask_info
}
