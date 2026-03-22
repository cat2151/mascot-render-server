//! rawpsd is a library that handles loading PSD data into a list of minimally-processed in-memory structs. It does not have any opinions about what features PSD files should or do use, or how to interpret those features. Compressed data is decompressed, and some redundant pieces of data like ascii and unicode names stored together are only returned once instead of twice, but aside from things like that, rawpsd is minimally opinionated and tries to just tell you what the PSD file itself says. For example, strings are left as strings instead of being transformed into enums.
//!
//! Comparison with other crates:
//! - `psd`: The `psd` crate's API makes it impossible to figure out the exact layer group hierarchy, so you can only use it on very simple PSDs.
//! - `zune-psd`: Doesn't actually support the psd format, just gets the embedded thumbnail.
//!
//! rawpsd draws a compatibility support line at Photoshop CS6, the last non-subscription version of Photoshop. Features only supported by newer versions are unlikely to be supported.
//!
//! rawpsd currently only supports 8-bit RGB, CMYK, and Grayscale PSDs. This is the vast majority of PSD files that can be found in the wild. It does not yet support the large document PSB format variant.
//!
//! rawpsd's docs do not document the entire PSD format, not even its capabilities. You will need to occasionally reference <https://www.adobe.com/devnet-apps/photoshop/fileformatashtml/> and manually poke at PSD files in a hex editor to take full advantage of rawpsd.
//!
//! You want [parse_layer_records] and [parse_psd_metadata].
//!
//! Example:
//!
//!```rs
//!let data = std::fs::read("data/test.psd").expect("Failed to open test.psd");
//!
//!if let Ok(layers) = parse_layer_records(&data)
//!{
//!    for mut layer in layers
//!    {
//!        // Don't spew tons of image data bytes to stdout; we just want to see the metadata.
//!        layer.image_data_rgba = vec!();
//!        layer.image_data_k = vec!();
//!        layer.image_data_mask = vec!();
//!        println!("{:?}", layer);
//!    }
//!}
//!```

#![allow(clippy::field_reassign_with_default)]
#![allow(clippy::manual_range_contains)]
#![allow(clippy::vec_init_then_push)]
#![cfg_attr(not(any(test, feature = "serde_support", feature = "debug_spew")), no_std)]

extern crate alloc;

mod cursor;
mod descriptor;
mod extra_data;
mod layer_parse;
mod layer_types;
mod parse_support;

pub use descriptor::DescItem;
pub use layer_parse::parse_layer_records;
pub use layer_types::{BlendModeDocs, LayerInfo, MaskInfo, PsdMetadata};
pub use parse_support::{append_img_data, copy_img_data, parse_psd_metadata};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        let data = std::fs::read("data/test.psd").expect("Failed to open test.psd");

        if let Ok(layers) = parse_layer_records(&data) {
            for mut layer in layers {
                layer.image_data_rgba = vec![];
                layer.image_data_k = vec![];
                layer.image_data_mask = vec![];
                println!("{:?}", layer);
            }
        }

        println!("-----");

        let data = std::fs::read("data/test2.psd").expect("Failed to open test2.psd");

        if let Ok(layers) = parse_layer_records(&data) {
            for mut layer in layers {
                layer.image_data_rgba = vec![];
                layer.image_data_k = vec![];
                layer.image_data_mask = vec![];
                println!("{:?}", layer);
            }
        }
    }
}
