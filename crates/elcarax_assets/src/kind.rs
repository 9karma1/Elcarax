use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AssetKind {
    Audio,
    Folder,
    Image,
    Material,
    Model,
    Scene,
    Script,
    Text,
    Unknown,
}

impl AssetKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Folder => "Folder",
            Self::Scene => "Scene",
            Self::Image => "Image",
            Self::Audio => "Audio",
            Self::Model => "Model",
            Self::Script => "Script",
            Self::Material => "Material",
            Self::Text => "Text",
            Self::Unknown => "Unknown",
        }
    }
}

pub fn detect_kind_from_path(path: &Path, is_directory: bool) -> AssetKind {
    if is_directory {
        return AssetKind::Folder;
    }
    detect_kind_from_extension(path.extension().and_then(|ext| ext.to_str()))
}

pub fn detect_kind_from_extension(extension: Option<&str>) -> AssetKind {
    let Some(extension) = extension else {
        return AssetKind::Unknown;
    };
    let extension = extension.to_ascii_lowercase();
    match extension.as_str() {
        "scene" => AssetKind::Scene,
        "png" | "jpg" | "jpeg" | "webp" | "bmp" | "gif" => AssetKind::Image,
        "wav" | "ogg" | "mp3" | "flac" => AssetKind::Audio,
        "glb" | "gltf" | "obj" | "fbx" => AssetKind::Model,
        "rs" | "lua" | "gd" | "js" | "ts" => AssetKind::Script,
        "material" | "mat" => AssetKind::Material,
        "md" | "txt" | "json" | "toml" | "yaml" | "yml" => AssetKind::Text,
        _ => AssetKind::Unknown,
    }
}
