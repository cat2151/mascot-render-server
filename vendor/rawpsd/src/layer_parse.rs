use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

use crate::cursor::SliceCursor;
use crate::extra_data::apply_layer_extra_data;
use crate::layer_types::LayerInfo;
use crate::parse_support::{
    append_img_data, copy_img_data, parse_psd_metadata, read_b4, read_i32, read_mask_info,
    read_u16, read_u32, read_u8,
};

/// Parses the layer records out of a PSD file, producing a bottom-to-top list.
///
/// PSD data is compressed and poorly-ordered, so it's very rare to benefit from streaming loading, even for performance. Therefore, to keep things simple, the input is a slice instead of a streaming trait.
///
/// PSD doesn't store its layer data in a tree; instead, it uses start-of-group and end-of-group nodes in a list to indicate tree structure.
///
/// On failure, returns all the layers that have been parsed *so far, from the bottom* (PSD files are stored bottom-up), and a string describing the error. The incomplete list of parsed layers is unlikely to be useful for any application, but it may be useful for debugging.
///
/// You will need to use both this and [crate::parse_psd_metadata].
pub fn parse_layer_records(data: &[u8]) -> Result<Vec<LayerInfo>, (Vec<LayerInfo>, String)> {
    let mut layers = Vec::new();
    let ret = parse_layer_records_impl(data, &mut layers);
    match ret {
        Ok(_) => Ok(layers),
        Err(err) => Err((layers, err)),
    }
}

fn parse_layer_records_impl(data: &[u8], layers: &mut Vec<LayerInfo>) -> Result<(), String> {
    let metadata = parse_psd_metadata(data)?;
    if metadata.depth != 8 {
        return Err(
            "Only PSDs in 8-bit RGB, CMYK, or Grayscale mode are currently supported.".to_string(),
        );
    }
    if metadata.color_mode != 1 && metadata.color_mode != 3 && metadata.color_mode != 4 {
        return Err(
            "Only PSDs in 8-bit RGB, CMYK, or Grayscale mode are currently supported.".to_string(),
        );
    }

    let mut cursor = SliceCursor::new(data);
    cursor.set_position(26);

    let color_mode_length = read_u32(&mut cursor)? as u64;
    cursor.set_position(cursor.position() + color_mode_length);

    let image_resources_length = read_u32(&mut cursor)? as u64;
    cursor.set_position(cursor.position() + image_resources_length);

    let layer_mask_info_length = read_u32(&mut cursor)? as u64;
    let _layer_mask_info_end = cursor.position() + layer_mask_info_length;

    let layer_info_length = read_u32(&mut cursor)? as u64;
    let _layer_info_end = cursor.position() + layer_info_length;

    let layer_count = read_u16(&mut cursor)? as i16;
    let layer_count = layer_count.abs();

    #[cfg(feature = "debug_spew")]
    println!("starting at {:X}", cursor.position());

    let mut idata_c = SliceCursor::new(data);
    idata_c.set_position(cursor.position());

    for _ in 0..layer_count {
        read_i32(&mut idata_c)?;
        read_i32(&mut idata_c)?;
        read_i32(&mut idata_c)?;
        read_i32(&mut idata_c)?;
        let image_channel_count = read_u16(&mut idata_c)? as u64;
        idata_c.set_position(idata_c.position() + 6 * image_channel_count + 4 + 4 + 4);
        let idat_len = read_u32(&mut idata_c)? as u64;
        idata_c.set_position(idata_c.position() + idat_len);
    }

    for _ in 0..layer_count {
        let top = read_i32(&mut cursor)?;
        let left = read_i32(&mut cursor)?;
        let bottom = read_i32(&mut cursor)?;
        let right = read_i32(&mut cursor)?;

        let x = left;
        let y = top;
        let w = (right - left) as u32;
        let h = (bottom - top) as u32;
        let image_channel_count = read_u16(&mut cursor)?;
        let channel_info_start = cursor.position();

        cursor.set_position(channel_info_start);
        let mut image_data_rgba = vec![255u8; (w * h * 4) as usize];
        let mut image_data_k = vec![];
        let mut image_data_mask = vec![];

        let mut has_g = false;
        let mut has_b = false;
        let mut has_a = false;
        let mut aux_count = 0;
        let mut cdat_cursor = cursor.clone();

        for _ in 0..image_channel_count {
            read_u16(&mut cursor)?;
            read_u32(&mut cursor)?;
        }

        let blend_mode_signature = read_b4(&mut cursor)?;
        if blend_mode_signature != [0x38, 0x42, 0x49, 0x4D] {
            return Err("Invalid blend mode magic signature".to_string());
        }

        let blend_mode_key = read_b4(&mut cursor)?;
        let blend_mode = String::from_utf8_lossy(&blend_mode_key).to_string();
        let opacity = read_u8(&mut cursor)? as f32 / 255.0;
        #[cfg(feature = "debug_spew")]
        println!("opacity: {}", opacity * 100.0);
        let clipping = read_u8(&mut cursor)?;
        let flags = read_u8(&mut cursor)?;
        let _filler = read_u8(&mut cursor)?;

        let exdat_len = read_u32(&mut cursor)? as u64;
        let exdat_start = cursor.position();
        let mask_info = read_mask_info(&mut cursor)?;

        for _ in 0..image_channel_count {
            let channel_id = read_u16(&mut cdat_cursor)? as i16;
            has_g |= channel_id == 1;
            has_b |= channel_id == 2;
            has_a |= channel_id == -1;
            let channel_length = read_u32(&mut cdat_cursor)? as usize;
            #[cfg(feature = "debug_spew")]
            println!(
                "channel... {} {} at 0x{:X}",
                channel_id,
                channel_length,
                idata_c.position()
            );
            if (-1..=2).contains(&channel_id) {
                let pos = if channel_id >= 0 { channel_id } else { 3 } as usize;
                #[cfg(feature = "debug_spew")]
                println!("{} {} {} {}", w, h, pos, channel_length);
                if channel_length > 2 {
                    let progress = copy_img_data(
                        idata_c.take_rest().buf,
                        &mut image_data_rgba[pos..],
                        4,
                        channel_length as u64,
                        h as u64,
                    )?;
                    idata_c.pos += progress;
                } else {
                    idata_c.set_position(idata_c.position() + 2);
                }
            } else if channel_id == 3 {
                if channel_length > 2 {
                    let progress = append_img_data(
                        idata_c.take_rest().buf,
                        &mut image_data_k,
                        channel_length as u64,
                        h as u64,
                    )?;
                    idata_c.pos += progress;
                } else {
                    idata_c.set_position(idata_c.position() + 2);
                }
            } else {
                #[cfg(feature = "debug_spew")]
                println!("mask... {} {} {}", mask_info.w, mask_info.h, channel_length);
                aux_count += 1;
                if aux_count > 1 || mask_info.h == 0 {
                    idata_c.set_position(idata_c.position() + channel_length as u64);
                } else if channel_length > 2 {
                    #[cfg(feature = "debug_spew")]
                    println!("adding mask data...");
                    let progress = append_img_data(
                        idata_c.take_rest().buf,
                        &mut image_data_mask,
                        channel_length as u64,
                        mask_info.h as u64,
                    )?;
                    idata_c.pos += progress;
                } else {
                    idata_c.set_position(idata_c.position() + 2);
                }
            }
        }

        let blendat_len = read_u32(&mut cursor)? as u64;
        cursor.set_position(cursor.position() + blendat_len);

        let mut name_len = read_u8(&mut cursor)?;
        let orig_namelen = name_len;
        while (name_len + 1) % 4 != 0 {
            name_len += 1;
        }
        let mut name = vec![0; name_len as usize];
        cursor.read_exact(&mut name[..]).map_err(|x| x.to_string())?;
        let name = String::from_utf8_lossy(&name[..orig_namelen as usize]).to_string();

        let mut layer = LayerInfo {
            name,
            opacity,
            fill_opacity: 1.0,
            blend_mode,
            x,
            y,
            w,
            h,
            image_channel_count,
            image_data_rgba,
            image_data_k,
            image_data_has_g: has_g,
            image_data_has_b: has_b,
            image_data_has_a: has_a,
            mask_channel_count: aux_count,
            mask_info,
            image_data_mask,
            group_expanded: false,
            group_opener: false,
            group_closer: false,
            funny_flag: false,
            is_clipped: clipping != 0,
            is_alpha_locked: (flags & 1) != 0,
            is_visible: (flags & 2) == 0,
            adjustment_type: "".to_string(),
            adjustment_info: vec![],
            adjustment_desc: None,
            effects_desc: None,
        };

        while cursor.position() < exdat_start + exdat_len {
            let sig = read_b4(&mut cursor)?;
            if sig != [0x38, 0x42, 0x49, 0x4D] {
                return Err("Invalid extended data magic signature".to_string());
            }

            let name = read_b4(&mut cursor)?;
            let name = String::from_utf8_lossy(&name).to_string();
            let len = read_u32(&mut cursor)? as u64;
            let start = cursor.position();

            #[cfg(feature = "debug_spew")]
            println!("reading metadata.... {}", name.as_str());
            apply_layer_extra_data(name.as_str(), &mut cursor, &mut layer)?;
            cursor.set_position(start + len);
        }

        if cursor.position() != exdat_start + exdat_len {
            return Err("Desynchronized while reading or skipping extra data".to_string());
        }

        #[cfg(feature = "debug_spew")]
        println!("added layer with name {}", layer.name);
        layers.push(layer);
    }

    Ok(())
}
