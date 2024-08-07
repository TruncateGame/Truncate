const tiles = {
    NONE: 0,
    DEBUG: 1,
    BASE_GRASS: 2,
    GRASS_0_WIND_0: 3,
    GRASS_0_WIND_1: 4,
    GRASS_0_WIND_2: 5,
    GRASS_0_WIND_3: 6,
    GRASS_1_WIND_0: 7,
    GRASS_1_WIND_1: 8,
    GRASS_1_WIND_2: 9,
    GRASS_1_WIND_3: 10,
    GRASS_2_WIND_0: 11,
    GRASS_2_WIND_1: 12,
    GRASS_2_WIND_2: 13,
    GRASS_2_WIND_3: 14,
    CHECKERBOARD_NW: 15,
    CHECKERBOARD_NE: 16,
    CHECKERBOARD_SW: 17,
    CHECKERBOARD_SE: 18,
    BASE_WATER: 19,
    WATER_WITH_LAND_SE: 20,
    WATER_WITH_LAND_S: 21,
    WATER_WITH_LAND_SW: 22,
    WATER_WITH_LAND_E: 23,
    WATER_WITH_LAND_W: 24,
    WATER_WITH_LAND_NE: 25,
    WATER_WITH_LAND_N: 26,
    WATER_WITH_LAND_NW: 27,
    LAND_WITH_WATER_SE: 28,
    LAND_WITH_WATER_SW: 29,
    LAND_WITH_WATER_NE: 30,
    LAND_WITH_WATER_NW: 31,
    ISLAND_NW: 32,
    ISLAND_NE: 33,
    ISLAND_SW: 34,
    ISLAND_SE: 35,
    LAKE_NW: 36,
    LAKE_NE: 37,
    LAKE_SW: 38,
    LAKE_SE: 39,
    MISC_COBBLE_0: 40,
    MISC_COBBLE_1: 41,
    MISC_COBBLE_2: 42,
    MISC_COBBLE_3: 43,
    MISC_COBBLE_4: 44,
    MISC_COBBLE_5: 45,
    MISC_COBBLE_6: 46,
    MISC_COBBLE_7: 47,
    MISC_COBBLE_8: 48,
    HOUSE_0: 49,
    ROOF_0: 50,
    HOUSE_1: 51,
    ROOF_1: 52,
    HOUSE_2: 53,
    ROOF_2: 54,
    HOUSE_3: 55,
    ROOF_3: 56,
    SMOKE_0: 57,
    SMOKE_1: 58,
    SMOKE_2: 59,
    SMOKE_3: 60,
    SMOKE_4: 61,
    BUSH_0: 62,
    BUSH_FLOWERS_0: 63,
    BUSH_1: 64,
    BUSH_FLOWERS_1: 65,
    BUSH_2: 66,
    BUSH_FLOWERS_2: 67,
    WHEAT_WIND_0: 68,
    WHEAT_WIND_1: 69,
    WELL: 70,
    NORTH_DOCK_NW: 71,
    NORTH_DOCK_NE: 72,
    NORTH_DOCK_SW: 73,
    NORTH_DOCK_SE: 74,
    NORTH_DOCK_SAIL_WIND_0_NW: 75,
    NORTH_DOCK_SAIL_WIND_0_NE: 76,
    NORTH_DOCK_SAIL_WIND_1_NW: 77,
    NORTH_DOCK_SAIL_WIND_1_NE: 78,
    NORTH_DOCK_SAIL_WIND_2_NW: 79,
    NORTH_DOCK_SAIL_WIND_2_NE: 80,
    SOUTH_DOCK_NW: 81,
    SOUTH_DOCK_NE: 82,
    SOUTH_DOCK_SW: 83,
    SOUTH_DOCK_SE: 84,
    SOUTH_DOCK_SAIL_WIND_0_NW: 85,
    SOUTH_DOCK_SAIL_WIND_0_SW: 86,
    SOUTH_DOCK_SAIL_WIND_1_NW: 87,
    SOUTH_DOCK_SAIL_WIND_1_SW: 88,
    SOUTH_DOCK_SAIL_WIND_2_NW: 89,
    SOUTH_DOCK_SAIL_WIND_2_SW: 90,
    EAST_DOCK_NW: 91,
    EAST_DOCK_NE: 92,
    EAST_DOCK_SW: 93,
    EAST_DOCK_SE: 94,
    EAST_DOCK_SAIL_WIND_0_NW: 95,
    EAST_DOCK_SAIL_WIND_0_NE: 96,
    EAST_DOCK_SAIL_WIND_0_SW: 97,
    EAST_DOCK_SAIL_WIND_0_SE: 98,
    EAST_DOCK_SAIL_WIND_1_NW: 99,
    EAST_DOCK_SAIL_WIND_1_NE: 100,
    EAST_DOCK_SAIL_WIND_1_SW: 101,
    EAST_DOCK_SAIL_WIND_1_SE: 102,
    EAST_DOCK_SAIL_WIND_2_NW: 103,
    EAST_DOCK_SAIL_WIND_2_NE: 104,
    EAST_DOCK_SAIL_WIND_2_SW: 105,
    EAST_DOCK_SAIL_WIND_2_SE: 106,
    WEST_DOCK_NW: 107,
    WEST_DOCK_NE: 108,
    WEST_DOCK_SW: 109,
    WEST_DOCK_SE: 110,
    WEST_DOCK_SAIL_WIND_0_NW: 111,
    WEST_DOCK_SAIL_WIND_0_NE: 112,
    WEST_DOCK_SAIL_WIND_1_NW: 113,
    WEST_DOCK_SAIL_WIND_1_NE: 114,
    WEST_DOCK_SAIL_WIND_2_NW: 115,
    WEST_DOCK_SAIL_WIND_2_NE: 116,
    FLOATING_DOCK_NW: 117,
    FLOATING_DOCK_NE: 118,
    FLOATING_DOCK_SW: 119,
    FLOATING_DOCK_SE: 120,
    FLOATING_DOCK_SAIL_WIND_0_NW: 121,
    FLOATING_DOCK_SAIL_WIND_0_NE: 122,
    FLOATING_DOCK_SAIL_WIND_1_NW: 123,
    FLOATING_DOCK_SAIL_WIND_1_NE: 124,
    FLOATING_DOCK_SAIL_WIND_2_NW: 125,
    FLOATING_DOCK_SAIL_WIND_2_NE: 126,
    GAME_PIECE_NW: 127,
    GAME_PIECE_NE: 128,
    GAME_PIECE_SW: 129,
    GAME_PIECE_SE: 130,
    HIGHLIGHT_NW: 131,
    HIGHLIGHT_NE: 132,
    HIGHLIGHT_SW: 133,
    HIGHLIGHT_SE: 134,
    GAME_PIECE_N: 135,
    GAME_PIECE_S: 136,
    GAME_PIECE_GRASS_0_SW: 137,
    GAME_PIECE_GRASS_0_SE: 138,
    GAME_PIECE_GRASS_1_SW: 139,
    GAME_PIECE_GRASS_1_SE: 140,
    GAME_PIECE_GRASS_2_SW: 141,
    GAME_PIECE_GRASS_2_SE: 142,
    GAME_PIECE_GRASS_3_SW: 143,
    GAME_PIECE_GRASS_3_SE: 144,
    GAME_PIECE_CRACKS_0_SW: 145,
    GAME_PIECE_CRACKS_0_SE: 146,
    GAME_PIECE_CRACKS_1_NW: 147,
    GAME_PIECE_CRACKS_1_NE: 148,
    GAME_PIECE_CRACKS_1_SE: 149,
    GAME_PIECE_CRACKS_2_NE: 150,
    GAME_PIECE_CRACKS_2_SE: 151,
    GAME_PIECE_CRACKS_3_NW: 152,
    GAME_PIECE_CRACKS_3_SW: 153,
    GAME_PIECE_CRACKS_4_NW: 154,
    GAME_PIECE_RUBBLE_0_NW: 155,
    GAME_PIECE_RUBBLE_0_NE: 156,
    GAME_PIECE_RUBBLE_0_SW: 157,
    GAME_PIECE_RUBBLE_0_SE: 158,
    GAME_PIECE_RUBBLE_1_NW: 159,
    GAME_PIECE_RUBBLE_1_NE: 160,
    GAME_PIECE_RUBBLE_1_SW: 161,
    GAME_PIECE_RUBBLE_1_SE: 162,
    GAME_PIECE_RUBBLE_2_NW: 163,
    GAME_PIECE_RUBBLE_2_NE: 164,
    GAME_PIECE_RUBBLE_2_SW: 165,
    GAME_PIECE_RUBBLE_2_SE: 166,
    SELECTION_SPINNER_1_NW: 167,
    SELECTION_SPINNER_1_NE: 168,
    SELECTION_SPINNER_1_SW: 169,
    SELECTION_SPINNER_1_SE: 170,
    SELECTION_SPINNER_2_NW: 171,
    SELECTION_SPINNER_2_NE: 172,
    SELECTION_SPINNER_2_SW: 173,
    SELECTION_SPINNER_2_SE: 174,
    SELECTION_SPINNER_3_NW: 175,
    SELECTION_SPINNER_3_NE: 176,
    SELECTION_SPINNER_3_SW: 177,
    SELECTION_SPINNER_3_SE: 178,
    SELECTION_SPINNER_4_NW: 179,
    SELECTION_SPINNER_4_NE: 180,
    SELECTION_SPINNER_4_SW: 181,
    SELECTION_SPINNER_4_SE: 182,
    DIALOG_NW: 183,
    DIALOG_N: 184,
    DIALOG_NE: 185,
    DIALOG_W: 186,
    DIALOG_CENTER: 187,
    DIALOG_E: 188,
    DIALOG_SW: 189,
    DIALOG_S: 190,
    DIALOG_SE: 191,
    INFO_BUTTON_NW: 192,
    INFO_BUTTON_NE: 193,
    INFO_BUTTON_SW: 194,
    INFO_BUTTON_SE: 195,
    CLOSE_BUTTON_NW: 196,
    CLOSE_BUTTON_NE: 197,
    CLOSE_BUTTON_SW: 198,
    CLOSE_BUTTON_SE: 199,
    BUTTON_NOTIFICATION_SE: 200,
    BARE_CLOSE_BUTTON_NW: 201,
    BARE_CLOSE_BUTTON_NE: 202,
    BARE_CLOSE_BUTTON_SW: 203,
    BARE_CLOSE_BUTTON_SE: 204,
    RESIGN_BUTTON_NW: 205,
    RESIGN_BUTTON_NE: 206,
    RESIGN_BUTTON_SW: 207,
    RESIGN_BUTTON_SE: 208,
    TRI_EAST_BUTTON_NW: 209,
    TRI_EAST_BUTTON_NE: 210,
    TRI_EAST_BUTTON_SW: 211,
    TRI_EAST_BUTTON_SE: 212,
    SKIP_NEXT_BUTTON_NW: 213,
    SKIP_NEXT_BUTTON_NE: 214,
    SKIP_NEXT_BUTTON_SW: 215,
    SKIP_NEXT_BUTTON_SE: 216,
    TRI_NORTH_BUTTON_NW: 217,
    TRI_NORTH_BUTTON_NE: 218,
    TRI_NORTH_BUTTON_SW: 219,
    TRI_NORTH_BUTTON_SE: 220,
    TRI_SOUTH_BUTTON_NW: 221,
    TRI_SOUTH_BUTTON_NE: 222,
    TRI_SOUTH_BUTTON_SW: 223,
    TRI_SOUTH_BUTTON_SE: 224,
    SKIP_PREV_BUTTON_NW: 225,
    SKIP_PREV_BUTTON_NE: 226,
    SKIP_PREV_BUTTON_SW: 227,
    SKIP_PREV_BUTTON_SE: 228,
    DICT_BUTTON_NW: 229,
    DICT_BUTTON_NE: 230,
    DICT_BUTTON_SW: 231,
    DICT_BUTTON_SE: 232,
    TOWN_BUTTON_NW: 233,
    TOWN_BUTTON_NE: 234,
    TOWN_BUTTON_SW: 235,
    TOWN_BUTTON_SE: 236,
    TOWN_BUTTON_ROOF_NW: 237,
    TOWN_BUTTON_ROOF_NE: 238,
    DOCK_BUTTON_NW: 239,
    DOCK_BUTTON_NE: 240,
    DOCK_BUTTON_SW: 241,
    DOCK_BUTTON_SE: 242,
    DOCK_BUTTON_SAIL_NW: 243,
    DOCK_BUTTON_SAIL_NE: 244,
    TERRAIN_BUTTON_NW: 245,
    TERRAIN_BUTTON_NE: 246,
    TERRAIN_BUTTON_SW: 247,
    TERRAIN_BUTTON_SE: 248,
};

const quads = {
    CHECKERBOARD: [[tiles.CHECKERBOARD_NW, tiles.CHECKERBOARD_NE],[tiles.CHECKERBOARD_SW, tiles.CHECKERBOARD_SE]],
    WATER_WITH_LAND: [[tiles.WATER_WITH_LAND_NW, tiles.WATER_WITH_LAND_NE],[tiles.WATER_WITH_LAND_SW, tiles.WATER_WITH_LAND_SE]],
    LAND_WITH_WATER: [[tiles.LAND_WITH_WATER_NW, tiles.LAND_WITH_WATER_NE],[tiles.LAND_WITH_WATER_SW, tiles.LAND_WITH_WATER_SE]],
    ISLAND: [[tiles.ISLAND_NW, tiles.ISLAND_NE],[tiles.ISLAND_SW, tiles.ISLAND_SE]],
    LAKE: [[tiles.LAKE_NW, tiles.LAKE_NE],[tiles.LAKE_SW, tiles.LAKE_SE]],
    NORTH_DOCK: [[tiles.NORTH_DOCK_NW, tiles.NORTH_DOCK_NE],[tiles.NORTH_DOCK_SW, tiles.NORTH_DOCK_SE]],
    NORTH_DOCK_SAIL_WIND_0: [[tiles.NORTH_DOCK_SAIL_WIND_0_NW, tiles.NORTH_DOCK_SAIL_WIND_0_NE],[tiles.NONE, tiles.NONE]],
    NORTH_DOCK_SAIL_WIND_1: [[tiles.NORTH_DOCK_SAIL_WIND_1_NW, tiles.NORTH_DOCK_SAIL_WIND_1_NE],[tiles.NONE, tiles.NONE]],
    NORTH_DOCK_SAIL_WIND_2: [[tiles.NORTH_DOCK_SAIL_WIND_2_NW, tiles.NORTH_DOCK_SAIL_WIND_2_NE],[tiles.NONE, tiles.NONE]],
    SOUTH_DOCK: [[tiles.SOUTH_DOCK_NW, tiles.SOUTH_DOCK_NE],[tiles.SOUTH_DOCK_SW, tiles.SOUTH_DOCK_SE]],
    SOUTH_DOCK_SAIL_WIND_0: [[tiles.SOUTH_DOCK_SAIL_WIND_0_NW, tiles.NONE],[tiles.SOUTH_DOCK_SAIL_WIND_0_SW, tiles.NONE]],
    SOUTH_DOCK_SAIL_WIND_1: [[tiles.SOUTH_DOCK_SAIL_WIND_1_NW, tiles.NONE],[tiles.SOUTH_DOCK_SAIL_WIND_1_SW, tiles.NONE]],
    SOUTH_DOCK_SAIL_WIND_2: [[tiles.SOUTH_DOCK_SAIL_WIND_2_NW, tiles.NONE],[tiles.SOUTH_DOCK_SAIL_WIND_2_SW, tiles.NONE]],
    EAST_DOCK: [[tiles.EAST_DOCK_NW, tiles.EAST_DOCK_NE],[tiles.EAST_DOCK_SW, tiles.EAST_DOCK_SE]],
    EAST_DOCK_SAIL_WIND_0: [[tiles.EAST_DOCK_SAIL_WIND_0_NW, tiles.EAST_DOCK_SAIL_WIND_0_NE],[tiles.EAST_DOCK_SAIL_WIND_0_SW, tiles.EAST_DOCK_SAIL_WIND_0_SE]],
    EAST_DOCK_SAIL_WIND_1: [[tiles.EAST_DOCK_SAIL_WIND_1_NW, tiles.EAST_DOCK_SAIL_WIND_1_NE],[tiles.EAST_DOCK_SAIL_WIND_1_SW, tiles.EAST_DOCK_SAIL_WIND_1_SE]],
    EAST_DOCK_SAIL_WIND_2: [[tiles.EAST_DOCK_SAIL_WIND_2_NW, tiles.EAST_DOCK_SAIL_WIND_2_NE],[tiles.EAST_DOCK_SAIL_WIND_2_SW, tiles.EAST_DOCK_SAIL_WIND_2_SE]],
    WEST_DOCK: [[tiles.WEST_DOCK_NW, tiles.WEST_DOCK_NE],[tiles.WEST_DOCK_SW, tiles.WEST_DOCK_SE]],
    WEST_DOCK_SAIL_WIND_0: [[tiles.WEST_DOCK_SAIL_WIND_0_NW, tiles.WEST_DOCK_SAIL_WIND_0_NE],[tiles.NONE, tiles.NONE]],
    WEST_DOCK_SAIL_WIND_1: [[tiles.WEST_DOCK_SAIL_WIND_1_NW, tiles.WEST_DOCK_SAIL_WIND_1_NE],[tiles.NONE, tiles.NONE]],
    WEST_DOCK_SAIL_WIND_2: [[tiles.WEST_DOCK_SAIL_WIND_2_NW, tiles.WEST_DOCK_SAIL_WIND_2_NE],[tiles.NONE, tiles.NONE]],
    FLOATING_DOCK: [[tiles.FLOATING_DOCK_NW, tiles.FLOATING_DOCK_NE],[tiles.FLOATING_DOCK_SW, tiles.FLOATING_DOCK_SE]],
    FLOATING_DOCK_SAIL_WIND_0: [[tiles.FLOATING_DOCK_SAIL_WIND_0_NW, tiles.FLOATING_DOCK_SAIL_WIND_0_NE],[tiles.NONE, tiles.NONE]],
    FLOATING_DOCK_SAIL_WIND_1: [[tiles.FLOATING_DOCK_SAIL_WIND_1_NW, tiles.FLOATING_DOCK_SAIL_WIND_1_NE],[tiles.NONE, tiles.NONE]],
    FLOATING_DOCK_SAIL_WIND_2: [[tiles.FLOATING_DOCK_SAIL_WIND_2_NW, tiles.FLOATING_DOCK_SAIL_WIND_2_NE],[tiles.NONE, tiles.NONE]],
    GAME_PIECE: [[tiles.GAME_PIECE_NW, tiles.GAME_PIECE_NE],[tiles.GAME_PIECE_SW, tiles.GAME_PIECE_SE]],
    HIGHLIGHT: [[tiles.HIGHLIGHT_NW, tiles.HIGHLIGHT_NE],[tiles.HIGHLIGHT_SW, tiles.HIGHLIGHT_SE]],
    GAME_PIECE_GRASS_0: [[tiles.NONE, tiles.NONE],[tiles.GAME_PIECE_GRASS_0_SW, tiles.GAME_PIECE_GRASS_0_SE]],
    GAME_PIECE_GRASS_1: [[tiles.NONE, tiles.NONE],[tiles.GAME_PIECE_GRASS_1_SW, tiles.GAME_PIECE_GRASS_1_SE]],
    GAME_PIECE_GRASS_2: [[tiles.NONE, tiles.NONE],[tiles.GAME_PIECE_GRASS_2_SW, tiles.GAME_PIECE_GRASS_2_SE]],
    GAME_PIECE_GRASS_3: [[tiles.NONE, tiles.NONE],[tiles.GAME_PIECE_GRASS_3_SW, tiles.GAME_PIECE_GRASS_3_SE]],
    GAME_PIECE_CRACKS_0: [[tiles.NONE, tiles.NONE],[tiles.GAME_PIECE_CRACKS_0_SW, tiles.GAME_PIECE_CRACKS_0_SE]],
    GAME_PIECE_CRACKS_1: [[tiles.GAME_PIECE_CRACKS_1_NW, tiles.GAME_PIECE_CRACKS_1_NE],[tiles.NONE, tiles.GAME_PIECE_CRACKS_1_SE]],
    GAME_PIECE_CRACKS_2: [[tiles.NONE, tiles.GAME_PIECE_CRACKS_2_NE],[tiles.NONE, tiles.GAME_PIECE_CRACKS_2_SE]],
    GAME_PIECE_CRACKS_3: [[tiles.GAME_PIECE_CRACKS_3_NW, tiles.NONE],[tiles.GAME_PIECE_CRACKS_3_SW, tiles.NONE]],
    GAME_PIECE_CRACKS_4: [[tiles.GAME_PIECE_CRACKS_4_NW, tiles.NONE],[tiles.NONE, tiles.NONE]],
    GAME_PIECE_RUBBLE_0: [[tiles.GAME_PIECE_RUBBLE_0_NW, tiles.GAME_PIECE_RUBBLE_0_NE],[tiles.GAME_PIECE_RUBBLE_0_SW, tiles.GAME_PIECE_RUBBLE_0_SE]],
    GAME_PIECE_RUBBLE_1: [[tiles.GAME_PIECE_RUBBLE_1_NW, tiles.GAME_PIECE_RUBBLE_1_NE],[tiles.GAME_PIECE_RUBBLE_1_SW, tiles.GAME_PIECE_RUBBLE_1_SE]],
    GAME_PIECE_RUBBLE_2: [[tiles.GAME_PIECE_RUBBLE_2_NW, tiles.GAME_PIECE_RUBBLE_2_NE],[tiles.GAME_PIECE_RUBBLE_2_SW, tiles.GAME_PIECE_RUBBLE_2_SE]],
    SELECTION_SPINNER_1: [[tiles.SELECTION_SPINNER_1_NW, tiles.SELECTION_SPINNER_1_NE],[tiles.SELECTION_SPINNER_1_SW, tiles.SELECTION_SPINNER_1_SE]],
    SELECTION_SPINNER_2: [[tiles.SELECTION_SPINNER_2_NW, tiles.SELECTION_SPINNER_2_NE],[tiles.SELECTION_SPINNER_2_SW, tiles.SELECTION_SPINNER_2_SE]],
    SELECTION_SPINNER_3: [[tiles.SELECTION_SPINNER_3_NW, tiles.SELECTION_SPINNER_3_NE],[tiles.SELECTION_SPINNER_3_SW, tiles.SELECTION_SPINNER_3_SE]],
    SELECTION_SPINNER_4: [[tiles.SELECTION_SPINNER_4_NW, tiles.SELECTION_SPINNER_4_NE],[tiles.SELECTION_SPINNER_4_SW, tiles.SELECTION_SPINNER_4_SE]],
    DIALOG: [[tiles.DIALOG_NW, tiles.DIALOG_NE],[tiles.DIALOG_SW, tiles.DIALOG_SE]],
    INFO_BUTTON: [[tiles.INFO_BUTTON_NW, tiles.INFO_BUTTON_NE],[tiles.INFO_BUTTON_SW, tiles.INFO_BUTTON_SE]],
    CLOSE_BUTTON: [[tiles.CLOSE_BUTTON_NW, tiles.CLOSE_BUTTON_NE],[tiles.CLOSE_BUTTON_SW, tiles.CLOSE_BUTTON_SE]],
    BUTTON_NOTIFICATION: [[tiles.NONE, tiles.NONE],[tiles.NONE, tiles.BUTTON_NOTIFICATION_SE]],
    BARE_CLOSE_BUTTON: [[tiles.BARE_CLOSE_BUTTON_NW, tiles.BARE_CLOSE_BUTTON_NE],[tiles.BARE_CLOSE_BUTTON_SW, tiles.BARE_CLOSE_BUTTON_SE]],
    RESIGN_BUTTON: [[tiles.RESIGN_BUTTON_NW, tiles.RESIGN_BUTTON_NE],[tiles.RESIGN_BUTTON_SW, tiles.RESIGN_BUTTON_SE]],
    TRI_EAST_BUTTON: [[tiles.TRI_EAST_BUTTON_NW, tiles.TRI_EAST_BUTTON_NE],[tiles.TRI_EAST_BUTTON_SW, tiles.TRI_EAST_BUTTON_SE]],
    SKIP_NEXT_BUTTON: [[tiles.SKIP_NEXT_BUTTON_NW, tiles.SKIP_NEXT_BUTTON_NE],[tiles.SKIP_NEXT_BUTTON_SW, tiles.SKIP_NEXT_BUTTON_SE]],
    TRI_NORTH_BUTTON: [[tiles.TRI_NORTH_BUTTON_NW, tiles.TRI_NORTH_BUTTON_NE],[tiles.TRI_NORTH_BUTTON_SW, tiles.TRI_NORTH_BUTTON_SE]],
    TRI_SOUTH_BUTTON: [[tiles.TRI_SOUTH_BUTTON_NW, tiles.TRI_SOUTH_BUTTON_NE],[tiles.TRI_SOUTH_BUTTON_SW, tiles.TRI_SOUTH_BUTTON_SE]],
    SKIP_PREV_BUTTON: [[tiles.SKIP_PREV_BUTTON_NW, tiles.SKIP_PREV_BUTTON_NE],[tiles.SKIP_PREV_BUTTON_SW, tiles.SKIP_PREV_BUTTON_SE]],
    DICT_BUTTON: [[tiles.DICT_BUTTON_NW, tiles.DICT_BUTTON_NE],[tiles.DICT_BUTTON_SW, tiles.DICT_BUTTON_SE]],
    TOWN_BUTTON: [[tiles.TOWN_BUTTON_NW, tiles.TOWN_BUTTON_NE],[tiles.TOWN_BUTTON_SW, tiles.TOWN_BUTTON_SE]],
    TOWN_BUTTON_ROOF: [[tiles.TOWN_BUTTON_ROOF_NW, tiles.TOWN_BUTTON_ROOF_NE],[tiles.NONE, tiles.NONE]],
    DOCK_BUTTON: [[tiles.DOCK_BUTTON_NW, tiles.DOCK_BUTTON_NE],[tiles.DOCK_BUTTON_SW, tiles.DOCK_BUTTON_SE]],
    DOCK_BUTTON_SAIL: [[tiles.DOCK_BUTTON_SAIL_NW, tiles.DOCK_BUTTON_SAIL_NE],[tiles.NONE, tiles.NONE]],
    TERRAIN_BUTTON: [[tiles.TERRAIN_BUTTON_NW, tiles.TERRAIN_BUTTON_NE],[tiles.TERRAIN_BUTTON_SW, tiles.TERRAIN_BUTTON_SE]],
};

module.exports = async function () {
    return {
        ...tiles,
        QUAD: {
            ...quads,
        }
    }
}