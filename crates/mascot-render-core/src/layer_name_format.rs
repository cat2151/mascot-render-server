use crate::model::LayerKind;

pub fn is_mandatory_name(name: &str) -> bool {
    name.starts_with('!')
}

pub fn is_exclusive_name(name: &str) -> bool {
    name.starts_with('*')
}

pub fn is_toggleable_kind(kind: LayerKind) -> bool {
    kind != LayerKind::GroupClose
}

pub fn is_mandatory_kind(kind: LayerKind) -> bool {
    is_toggleable_kind(kind)
}

pub fn is_exclusive_kind(kind: LayerKind) -> bool {
    matches!(kind, LayerKind::Layer | LayerKind::GroupOpen)
}
