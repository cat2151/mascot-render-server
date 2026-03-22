use alloc::string::String;
use alloc::vec::Vec;

use crate::descriptor::Descriptor;

#[cfg(feature = "serde_support")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "serde_support")]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
/// Metadata about where a mask attached to an object physically is and how to interpret it.
///
/// Stability promise: Every field in this struct will always be public. This struct is safe to initialize with `{ ..., ..Default::default() }`.
///
/// This struct is general purpose enough that you might want to use it in your code directly instead of making a newtype. If you do, and you need to serde it, enable the `serde_support` feature. The serde format of this struct is not guaranteed to be stable between minor versions or patch versions; if you use the `serde_support` feature and need to ensure compatibility between different builds of your code, pin `rawpsd` to a specific exact version. Otherwise, make a newtype.
#[non_exhaustive]
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

#[cfg(not(feature = "serde_support"))]
#[derive(Clone, Debug, Default)]
/// Metadata about where a mask attached to an object physically is and how to interpret it.
///
/// Stability promise: Every field in this struct will always be public. This struct is safe to initialize with `{ ..., ..Default::default() }`.
///
/// This struct is general purpose enough that you might want to use it in your code directly instead of making a newtype. If you do, and you need to serde it, enable the `serde_support` feature. The serde format of this struct is not guaranteed to be stable between minor versions or patch versions; if you use the `serde_support` feature and need to ensure compatibility between different builds of your code, pin `rawpsd` to a specific exact version. Otherwise, make a newtype.
#[non_exhaustive]
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

/// Dummy struct to keep the main docs from being bloated. See [LayerInfo::blend_mode].
///
/// Normal blend modes:
/// ```text
///     "pass" => "Normal", // "Pass through" mode for groups. Does not behave as a normal blend mode. Affects composition pipeline behavior.
///     "norm" => "Normal",
///     "diss" => "Dissolve",
///     "dark" => "Darken",
///     "mul " => "Multiply",
///     "idiv" => "Color Burn",
///     "lbrn" => "Linear Burn",
///     "dkCl" => "Darken",
///     "lite" => "Lighten",
///     "scrn" => "Screen",
///     "div " => "Color Dodge",
///     "lddg" => "Add",
///     "lgCl" => "Lighten",
///     "over" => "Overlay",
///     "sLit" => "Soft Light",
///     "hLit" => "Hard Light",
///     "vLit" => "Vivid Light",
///     "lLit" => "Linear Light",
///     "pLit" => "Pin Light",
///     "hMix" => "Hard Mix",
///     "diff" => "Difference",
///     "smud" => "Exclusion",
///     "fsub" => "Subtract",
///     "fdiv" => "Divide",
///     "hue " => "Hue",
///     "sat " => "Saturation",
///     "colr" => "Color",
///     "lum " => "Luminosity",
///     _ => "Normal",
/// ```
/// Blend modes as found in certain Class Descriptor objects in certain effect/filter-related features:
/// ```text
///     "Nrml" => "Normal",
///     "Dslv" => "Dissolve",
///     "Drkn" => "Darken",
///     "Mltp" => "Multiply",
///     "CBrn" => "Color Burn",
///     "linearBurn" => "Linear Burn",
///     "darkerColor" => "Darken",
///     "Lghn" => "Lighten",
///     "Scrn" => "Screen",
///     "CDdg" => "Color Dodge",
///     "linearDodge" => "Add",
///     "lighterColor" => "Lighten",
///     "Ovrl" => "Overlay",
///     "SftL" => "Soft Light",
///     "HrdL" => "Hard Light",
///     "vividLight" => "Vivid Light",
///     "linearLight" => "Linear Light",
///     "pinLight" => "Pin Light",
///     "hardMix" => "Hard Mix",
///     "Dfrn" => "Difference",
///     "Xclu" => "Exclusion",
///     "blendSubtraction" => "Subtract",
///     "blendDivide" => "Divide",
///     "H   " => "Hue",
///     "Strt" => "Saturation",
///     "Clr " => "Color",
///     "Lmns" => "Luminosity",
///     _ => "Normal",
/// ```
pub struct BlendModeDocs {
    _no_init: core::marker::PhantomData<()>,
}

#[non_exhaustive]
#[derive(Clone, Debug, Default)]
/// Describes a single layer stack entry.
///
/// This data is very unorganized, and you should not use it directly in your application. You should move it out into your own types.
///
/// Returned from [crate::parse_layer_records].
pub struct LayerInfo {
    pub name: String,
    pub opacity: f32,
    pub fill_opacity: f32,
    /// Blend mode stored as a string. See [BlendModeDocs].
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

#[non_exhaustive]
#[derive(Debug, PartialEq)]
/// File-wide PSD header metadata.
///
/// Returned from [crate::parse_psd_metadata].
pub struct PsdMetadata {
    pub width: u32,
    pub height: u32,
    pub color_mode: u16,
    pub depth: u16,
    pub channel_count: u16,
}
