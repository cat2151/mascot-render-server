use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;

/// PSD Class Descriptor object data. Only used by certain PSD features.
///
/// Some PSD format features use a dynamic meta-object format instead of feature-specific data encoding; that information is what this type is responsible for holding.
#[non_exhaustive]
#[derive(Clone, Debug, Default)]
pub enum DescItem {
    #[allow(non_camel_case_types)]
    long(i32),
    #[allow(non_camel_case_types)]
    doub(f64),
    /// Float that carries unit system metadata. The string specifies the unit system. Examples of unit systems are "#Ang" and "#Pxl".
    UntF(String, f64),
    #[allow(non_camel_case_types)]
    bool(bool),
    TEXT(String),
    /// When rawpsd ran into an error while parsing the data that goes here: what kind of error was it?
    Err(String),
    /// Entire sub-object.
    Objc(Box<Descriptor>),
    #[allow(non_camel_case_types)]
    /// Enums, which are stringly typed in PSDs.
    _enum(String, String),
    /// Variable-length list.
    VlLs(Vec<DescItem>),
    /// Dummy non-data data.
    #[default]
    Xxx,
}

impl DescItem {
    /// Get the given item if the enum is of that kind, otherwise panic.
    pub fn long(&self) -> i32 {
        match self {
            DescItem::long(x) => *x,
            _ => panic!(),
        }
    }

    /// Get the given item if the enum is of that kind, otherwise panic.
    pub fn doub(&self) -> f64 {
        match self {
            DescItem::doub(x) => *x,
            _ => panic!(),
        }
    }

    /// Get the given item if the enum is of that kind, otherwise panic.
    pub fn bool(&self) -> bool {
        match self {
            DescItem::bool(x) => *x,
            _ => panic!(),
        }
    }

    /// Get the given item if the enum is of that kind, otherwise panic.
    pub fn _enum(&self) -> (String, String) {
        match self {
            DescItem::_enum(y, x) => (y.clone(), x.clone()),
            _ => panic!(),
        }
    }

    #[allow(non_snake_case)]
    /// Get the given item if the enum is of that kind, otherwise panic.
    pub fn UntF(&self) -> (String, f64) {
        match self {
            DescItem::UntF(y, x) => (y.clone(), *x),
            _ => panic!(),
        }
    }

    #[allow(non_snake_case)]
    /// Get the given item if the enum is of that kind, otherwise panic.
    pub fn Objc(&self) -> Box<Descriptor> {
        match self {
            DescItem::Objc(x) => x.clone(),
            _ => panic!(),
        }
    }

    #[allow(non_snake_case)]
    /// Get the given item if the enum is of that kind, otherwise panic.
    pub fn TEXT(&self) -> String {
        match self {
            DescItem::TEXT(x) => x.clone(),
            _ => panic!(),
        }
    }

    #[allow(non_snake_case)]
    /// Get the given item if the enum is of that kind, otherwise panic.
    pub fn VlLs(&self) -> Vec<DescItem> {
        match self {
            DescItem::VlLs(x) => x.clone(),
            _ => panic!(),
        }
    }
}

pub(crate) type Descriptor = (String, Vec<(String, DescItem)>);
