use epaint::{vec2, Color32, Rect, TextureHandle, Vec2};
use instant::Duration;
use truncate_core::{board::Coordinate, generation::BoardSeed, messages::RoomCode};

use crate::regions::active_game::HeaderType;

use super::Theme;

#[derive(Debug, Clone, PartialEq)]
pub struct HoveredRegion {
    pub rect: Rect,
    // If we're hovering the board, what coordinate is it?
    pub coord: Option<Coordinate>,
}

#[derive(Clone, Default)]
pub struct InteractionDepot {
    pub interactive: bool,
    pub dragging_tile: bool,
    pub released_tile: Option<(usize, Coordinate)>,
    pub selected_square_on_board: Option<Coordinate>,
    pub hovered_tile_on_board: Option<HoveredRegion>,
    pub playing_tile: Option<char>,
    pub selected_tile_in_hand: Option<usize>,
    pub highlight_tiles: Option<Vec<char>>,
    pub highlight_squares: Option<Vec<Coordinate>>,
}

#[derive(Clone, Default)]
pub struct RegionDepot {
    pub hand_total_rect: Option<Rect>,
    pub hand_companion_rect: Option<Rect>,
    pub headers_total_rect: Option<Rect>,
}

#[derive(Clone, Default)]
pub struct UIStateDepot {
    pub sidebar_toggled: bool,
    pub sidebar_visible: bool,
    pub unread_sidebar: bool,
    pub hand_visible: bool,
    pub is_mobile: bool,
    pub is_touch: bool,
    pub header_visible: HeaderType,
}

#[derive(Clone)]
pub struct BoardDepot {
    pub board_seed: Option<BoardSeed>,
    pub board_moved: bool,
    pub board_zoom: f32,
    pub board_pan: Vec2,
}

impl Default for BoardDepot {
    fn default() -> Self {
        Self {
            board_seed: None,
            board_moved: false,
            board_zoom: 1.0,
            board_pan: vec2(0.0, 0.0),
        }
    }
}

#[derive(Clone)]
pub struct TimingDepot {
    pub current_time: Duration,
    pub prev_to_next_turn: (Duration, Duration),
}

#[derive(Clone)]
pub struct GameplayDepot {
    pub room_code: RoomCode,
    pub player_number: u64,
    pub next_player_number: u64,
    pub error_msg: Option<String>,
    pub winner: Option<usize>,
}

#[derive(Clone)]
pub struct AestheticDepot {
    pub theme: Theme,
    pub qs_tick: u64,
    pub map_texture: TextureHandle,
    pub player_colors: Vec<Color32>,
}

#[derive(Clone)]
pub struct TruncateDepot {
    pub interactions: InteractionDepot,
    pub regions: RegionDepot,
    pub ui_state: UIStateDepot,
    pub board_info: BoardDepot,
    pub timing: TimingDepot,
    pub gameplay: GameplayDepot,
    pub aesthetics: AestheticDepot,
}
