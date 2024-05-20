use epaint::{vec2, Color32, Rect, TextureHandle, Vec2};
use instant::Duration;
use truncate_core::{
    board::{Coordinate, Square},
    generation::BoardSeed,
    messages::RoomCode,
    npc::scoring::NPCPersonality,
    reporting::Change,
};

use crate::regions::active_game::HeaderType;

use super::Theme;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HoveredRegion {
    pub rect: Rect,
    // If we're hovering the board, what coordinate is it?
    pub coord: Option<Coordinate>,
    // If we're hovering the board, what square is it?
    pub square: Option<Square>,
}

#[derive(Clone, Default)]
pub struct InteractionDepot {
    pub view_only: bool,
    pub released_tile: Option<(usize, Coordinate)>,
    pub dragging_tile_on_board: Option<(Coordinate, Square)>,
    pub selected_tile_on_board: Option<(Coordinate, Square)>,
    pub hovered_tile_on_board: Option<(Coordinate, Square)>,
    pub selected_square_on_board: Option<(Coordinate, Square)>,
    pub previous_selected_square_on_board: Option<(Coordinate, Square)>,
    pub hovered_unoccupied_square_on_board: Option<HoveredRegion>,
    pub hovered_occupied_square_on_board: Option<HoveredRegion>,
    pub playing_tile: Option<char>,
    pub hovered_tile_in_hand: Option<(usize, char)>,
    pub selected_tile_in_hand: Option<(usize, char)>,
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
    pub sidebar_hidden: bool,
    pub unread_sidebar: bool,
    pub hand_hidden: bool,
    pub is_mobile: bool,
    pub is_touch: bool,
    pub game_header: HeaderType,
    pub actions_menu_open: bool,
    pub dictionary_open: bool,
    pub dictionary_focused: bool,
    pub dictionary_opened_by_keyboard: bool,
    pub dictionary_showing_definition: bool,
    pub hand_height_last_frame: f32,
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

#[derive(Clone, Default)]
pub struct TimingDepot {
    pub current_time: Duration,
    pub last_turn_change: Duration,
    pub game_ends_at: Option<u64>,
}

#[derive(Clone)]
pub struct GameplayDepot {
    pub room_code: RoomCode,
    pub player_number: u64,
    pub next_player_number: Option<u64>,
    pub error_msg: Option<String>,
    pub winner: Option<usize>,
    pub changes: Vec<Change>,
    pub last_battle_origin: Option<Coordinate>,
    pub npc: Option<NPCPersonality>,
    pub remaining_turns: Option<u64>,
}

#[derive(Clone)]
pub struct AestheticDepot {
    pub theme: Theme,
    pub qs_tick: u64,
    pub map_texture: TextureHandle,
    pub player_colors: Vec<Color32>,
    pub destruction_tick: f32,
    pub destruction_duration: f32,
}

#[derive(Clone, Default)]
pub struct AudioDepot {
    pub muted: bool,
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
    pub audio: AudioDepot,
}
