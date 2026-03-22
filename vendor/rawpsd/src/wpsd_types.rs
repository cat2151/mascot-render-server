use std::io::Cursor;
use std::io::Read;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default)]
pub enum DescItem {
    #[allow(non_camel_case_types)]
    long(i32),
    #[allow(non_camel_case_types)]
    doub(f64),
    UntF(String, f64),
    #[allow(non_camel_case_types)]
    bool(bool),
    TEXT(String),
    Err(String),
    Objc(Box<Descriptor>),
    #[allow(non_camel_case_types)]
    r#enum(String, String),
    VlLs(Vec<DescItem>),
    #[default]
    Xxx,
}

impl DescItem {
    pub fn long(&self) -> i32 {
        match self {
            DescItem::long(x) => *x,
            _ => panic!(),
        }
    }

    pub fn doub(&self) -> f64 {
        match self {
            DescItem::doub(x) => *x,
            _ => panic!(),
        }
    }

    pub fn bool(&self) -> bool {
        match self {
            DescItem::bool(x) => *x,
            _ => panic!(),
        }
    }

    pub fn r#enum(&self) -> (String, String) {
        match self {
            DescItem::r#enum(y, x) => (y.clone(), x.clone()),
            _ => panic!(),
        }
    }

    #[allow(non_snake_case)]
    pub fn UntF(&self) -> (String, f64) {
        match self {
            DescItem::UntF(y, x) => (y.clone(), *x),
            _ => panic!(),
        }
    }

    #[allow(non_snake_case)]
    pub fn Objc(&self) -> Box<Descriptor> {
        match self {
            DescItem::Objc(x) => x.clone(),
            _ => panic!(),
        }
    }

    #[allow(non_snake_case)]
    pub fn TEXT(&self) -> String {
        match self {
            DescItem::TEXT(x) => x.clone(),
            _ => panic!(),
        }
    }

    #[allow(non_snake_case)]
    pub fn VlLs(&self) -> Vec<DescItem> {
        match self {
            DescItem::VlLs(x) => x.clone(),
            _ => panic!(),
        }
    }
}

pub(crate) type Descriptor = (String, Vec<(String, DescItem)>);

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MaskInfo {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
    pub default_color: u8,
    pub relative: bool,
    pub disabled: bool,
    pub invert: bool,
}

#[derive(Clone, Debug, Default)]
pub struct LayerInfo {
    pub name: String,
    pub opacity: f32,
    pub fill_opacity: f32,
    pub blend_mode: String,
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
    pub image_channel_count: u16,
    pub image_data_rgba: Vec<u8>,
    pub image_data_k: Vec<u8>,
    pub image_data_has_g: bool,
    pub image_data_has_b: bool,
    pub image_data_has_a: bool,
    pub mask_channel_count: u16,
    pub mask_info: MaskInfo,
    pub image_data_mask: Vec<u8>,
    pub group_expanded: bool,
    pub group_opener: bool,
    pub group_closer: bool,
    pub funny_flag: bool,
    pub is_clipped: bool,
    pub is_alpha_locked: bool,
    pub is_visible: bool,
    pub adjustment_type: String,
    pub adjustment_info: Vec<f32>,
    pub adjustment_desc: Option<Descriptor>,
    pub effects_desc: Option<Descriptor>,
}

#[derive(Debug, PartialEq)]
pub struct PsdMetadata {
    pub width: u32,
    pub height: u32,
    pub color_mode: u16,
    pub depth: u16,
    pub channel_count: u16,
}
