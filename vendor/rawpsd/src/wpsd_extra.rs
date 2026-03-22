use std::collections::HashMap;
use std::io::Cursor;
use std::io::Read;

use crate::wpsd_io::{read_f64, read_i32, read_u16, read_u32, read_u8};
use crate::wpsd_types::{DescItem, Descriptor, LayerInfo};

pub(crate) fn read_descriptor(c: &mut Cursor<&[u8]>) -> Descriptor {
    let n = read_u32(c) as u64;
    c.set_position(c.position() + n * 2);

    let mut idlen = read_u32(c);
    if idlen == 0 {
        idlen = 4;
    }
    let mut id = vec![0; idlen as usize];
    c.read_exact(&mut id).unwrap();
    let id = String::from_utf8_lossy(&id).to_string();

    let mut data = vec![];
    let itemcount = read_u32(c);

    for _ in 0..itemcount {
        let mut namelen = read_u32(c);
        if namelen == 0 {
            namelen = 4;
        }
        let mut name = vec![0; namelen as usize];
        c.read_exact(&mut name).unwrap();
        let name = String::from_utf8_lossy(&name).to_string();
        data.push((name, read_key(c)));
    }

    (id, data)
}

fn read_key(c: &mut Cursor<&[u8]>) -> DescItem {
    let mut id = vec![0; 4];
    c.read_exact(&mut id).unwrap();
    let id = String::from_utf8_lossy(&id).to_string();

    match id.as_str() {
        "long" => DescItem::long(read_i32(c)),
        "doub" => DescItem::doub(read_f64(c)),
        "Objc" => DescItem::Objc(Box::new(read_descriptor(c))),
        "bool" => DescItem::bool(read_u8(c) != 0),
        "TEXT" => {
            let len = read_u32(c) as u64;
            let mut text = vec![0; len as usize];
            for i in 0..len {
                text[i as usize] = read_u16(c);
            }
            let text = String::from_utf16_lossy(&text)
                .trim_end_matches('\0')
                .to_string();
            DescItem::TEXT(text)
        }
        "UntF" => {
            let mut typ = vec![0; 4];
            c.read_exact(&mut typ).unwrap();
            let typ = String::from_utf8_lossy(&typ).to_string();
            DescItem::UntF(typ, read_f64(c))
        }
        "enum" => {
            let mut len = read_u32(c);
            if len == 0 {
                len = 4;
            }
            let mut name1 = vec![0; len as usize];
            c.read_exact(&mut name1).unwrap();
            let name1 = String::from_utf8_lossy(&name1).to_string();

            let mut len = read_u32(c);
            if len == 0 {
                len = 4;
            }
            let mut name2 = vec![0; len as usize];
            c.read_exact(&mut name2).unwrap();
            let name2 = String::from_utf8_lossy(&name2).to_string();
            DescItem::r#enum(name1, name2)
        }
        "VlLs" => {
            let len = read_u32(c);
            let mut ret = vec![];
            for _ in 0..len {
                ret.push(read_key(c));
            }
            DescItem::VlLs(ret)
        }
        _ => {
            println!("!!! errant descriptor subobject type... {}", id);
            DescItem::Err(id)
        }
    }
}

pub(crate) fn apply_layer_extra_data(
    name: &str,
    cursor: &mut Cursor<&[u8]>,
    layer: &mut LayerInfo,
) {
    match name {
        "lsct" => {
            let kind = read_u32(cursor) as u64;
            layer.group_expanded = kind == 1;
            layer.group_opener = kind == 1 || kind == 2;
            layer.group_closer = kind == 3;
            if kind == 1 || kind == 2 {
                println!("group opener!");
            }
            if kind == 3 {
                println!("group closer!");
            }
        }
        "luni" => {
            let len = read_u32(cursor) as u64;
            let mut name = vec![0; len as usize];
            for i in 0..len {
                name[i as usize] = read_u16(cursor);
            }
            layer.name = String::from_utf16_lossy(&name).to_string();
        }
        "tsly" => {
            let thing = read_u8(cursor);
            layer.funny_flag = thing == 0;
            println!("{}", layer.funny_flag);
        }
        "iOpa" => {
            layer.fill_opacity = read_u8(cursor) as f32 / 255.0;
        }
        "lfx2" => {
            assert!(read_u32(cursor) == 0);
            assert!(read_u32(cursor) == 16);
            layer.effects_desc = Some(read_descriptor(cursor));
        }
        "post" => {
            let data = vec![read_u16(cursor) as f32];
            layer.adjustment_type = name.to_string();
            layer.adjustment_info = data;
        }
        "nvrt" => {
            layer.adjustment_type = name.to_string();
            layer.adjustment_info = vec![];
        }
        "brit" => {
            let data = vec![
                read_u16(cursor) as f32,
                read_u16(cursor) as f32,
                read_u16(cursor) as f32,
                read_u8(cursor) as f32,
                1.0,
            ];
            layer.adjustment_type = name.to_string();
            layer.adjustment_info = data;
        }
        "thrs" => {
            let data = vec![read_u16(cursor) as f32];
            layer.adjustment_type = name.to_string();
            layer.adjustment_info = data;
        }
        "hue2" => {
            let mut data = vec![];
            read_u16(cursor);
            data.push(read_u8(cursor) as f32);
            read_u8(cursor);
            data.push(read_u16(cursor) as i16 as f32);
            data.push(read_u16(cursor) as i16 as f32);
            data.push(read_u16(cursor) as i16 as f32);
            data.push(read_u16(cursor) as i16 as f32);
            data.push(read_u16(cursor) as i16 as f32);
            data.push(read_u16(cursor) as i16 as f32);
            layer.adjustment_type = name.to_string();
            layer.adjustment_info = data;
        }
        "levl" => {
            let mut data = vec![];
            assert!(read_u16(cursor) == 2);
            for _ in 0..28 {
                data.push(read_u16(cursor) as f32 / 255.0);
                data.push(read_u16(cursor) as f32 / 255.0);
                data.push(read_u16(cursor) as f32 / 255.0);
                data.push(read_u16(cursor) as f32 / 255.0);
                data.push(read_u16(cursor) as f32 / 100.0);
            }
            layer.adjustment_type = name.to_string();
            layer.adjustment_info = data;
        }
        "curv" => {
            let mut data = vec![];
            read_u8(cursor);
            assert!(read_u16(cursor) == 1);
            let enabled = read_u32(cursor);
            for i in 0..32 {
                if (enabled & (1u32 << i)) != 0 {
                    let n = read_u16(cursor);
                    data.push(n as f32);
                    for _ in 0..n {
                        let y = read_u16(cursor) as f32 / 255.0;
                        data.push(read_u16(cursor) as f32 / 255.0);
                        data.push(y);
                    }
                } else {
                    data.push(0.0);
                }
            }
            layer.adjustment_type = name.to_string();
            layer.adjustment_info = data;
        }
        "blwh" => {
            assert!(read_u32(cursor) == 16);
            layer.adjustment_type = name.to_string();
            layer.adjustment_desc = Some(read_descriptor(cursor));
        }
        "CgEd" => {
            assert!(read_u32(cursor) == 16);
            let temp = read_descriptor(cursor).1;
            println!("{:?}", temp);
            let mut n = HashMap::new();
            for t in temp {
                n.insert(t.0, t.1);
            }
            println!("{:?}", n);
            let data = vec![
                n.get("Brgh").unwrap().long() as f32,
                n.get("Cntr").unwrap().long() as f32,
                n.get("means").unwrap().long() as f32,
                n.get("Lab ").unwrap().bool() as u8 as f32,
                n.get("useLegacy").unwrap().bool() as u8 as f32,
            ];
            println!("??????????? {:?}", data);
            layer.adjustment_info = data;
        }
        _ => {}
    }
}
