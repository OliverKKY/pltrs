#[derive(Clone, Debug)]
pub struct Theme {
    pub bg: super::scene::Color,
    pub fg: super::scene::Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            bg: super::scene::Color::WHITE,
            fg: super::scene::Color::BLACK,
        }
    }
}
