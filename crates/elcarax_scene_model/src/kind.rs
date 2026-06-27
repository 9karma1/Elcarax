#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SceneObjectKind {
    Audio,
    Camera,
    Character,
    Cube,
    Environment,
    Ground,
    Light,
    Mesh,
    Trigger,
    World,
}

impl SceneObjectKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Audio => "Audio",
            Self::Camera => "Camera",
            Self::Character => "Character",
            Self::Cube => "Cube",
            Self::Environment => "Environment",
            Self::Ground => "Ground",
            Self::Light => "Light",
            Self::Mesh => "Mesh",
            Self::Trigger => "Trigger",
            Self::World => "World",
        }
    }
}
