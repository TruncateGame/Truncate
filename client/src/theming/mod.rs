use epaint::{hex_color, Color32};

#[derive(Debug)]
pub struct InteractTheme {
    pub base: Color32,
    pub dark: Color32,
    pub light: Color32,
}

#[derive(Debug)]
pub struct Theme {
    pub friend: InteractTheme,
    pub enemy: InteractTheme,
    pub text: InteractTheme,
    pub selection: Color32,
    pub background: Color32,
    pub outlines: Color32,
    pub addition: Color32,
    pub modification: Color32,
    pub defeated: Color32,
    pub grid_size: f32,
    pub tile_margin: f32,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            friend: InteractTheme {
                base: hex_color!("#E1E6F4"),
                dark: hex_color!("#C3CEEA"),
                light: hex_color!("#FFFFFF"),
            },
            enemy: InteractTheme {
                base: hex_color!("#F7BDB6"),
                dark: hex_color!("#F39B91"),
                light: hex_color!("#FBDEDA"),
            },
            text: InteractTheme {
                base: hex_color!("#333333"),
                dark: hex_color!("#1E1E1E"),
                light: hex_color!("#6B6B6B"),
            },
            selection: hex_color!("#D78D1D"),
            background: hex_color!("#1E1E1E"),
            outlines: hex_color!("#9B9B9B"),
            addition: hex_color!("#9CC69B"),
            modification: hex_color!("#9055A2"),
            defeated: hex_color!("#944D5E"),
            grid_size: 50.0,
            tile_margin: 4.0,
        }
    }
}
