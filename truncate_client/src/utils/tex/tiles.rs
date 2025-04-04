#![cfg_attr(rustfmt, rustfmt_skip)]

#![allow(dead_code)]

use super::{Tex, TexQuad};

const fn t(tile: usize) -> Tex {
    Tex { tile, tint: None }
}

pub const MAX_TILE: usize = 404;
pub const NONE: Tex = t(0);
pub const DEBUG: Tex = t(1);
pub const BASE_GRASS: Tex = t(2);
pub const GRASS_0_WIND_0: Tex = t(3);
pub const GRASS_0_WIND_1: Tex = t(4);
pub const GRASS_0_WIND_2: Tex = t(5);
pub const GRASS_0_WIND_3: Tex = t(6);
pub const GRASS_1_WIND_0: Tex = t(7);
pub const GRASS_1_WIND_1: Tex = t(8);
pub const GRASS_1_WIND_2: Tex = t(9);
pub const GRASS_1_WIND_3: Tex = t(10);
pub const GRASS_2_WIND_0: Tex = t(11);
pub const GRASS_2_WIND_1: Tex = t(12);
pub const GRASS_2_WIND_2: Tex = t(13);
pub const GRASS_2_WIND_3: Tex = t(14);
pub const CHECKERBOARD_NW: Tex = t(15);
pub const CHECKERBOARD_NE: Tex = t(16);
pub const CHECKERBOARD_SW: Tex = t(17);
pub const CHECKERBOARD_SE: Tex = t(18);
pub const BASE_WATER_WAVES: Tex = t(19);
pub const BASE_WATER: Tex = t(20);
pub const WATER_WITH_LAND_SE: Tex = t(21);
pub const WATER_WITH_LAND_S: Tex = t(22);
pub const WATER_WITH_LAND_SW: Tex = t(23);
pub const WATER_WITH_LAND_E: Tex = t(24);
pub const WATER_WITH_LAND_W: Tex = t(25);
pub const WATER_WITH_LAND_NE: Tex = t(26);
pub const WATER_WITH_LAND_N: Tex = t(27);
pub const WATER_WITH_LAND_NW: Tex = t(28);
pub const LAND_WITH_WATER_SE: Tex = t(29);
pub const LAND_WITH_WATER_SW: Tex = t(30);
pub const LAND_WITH_WATER_NE: Tex = t(31);
pub const LAND_WITH_WATER_NW: Tex = t(32);
pub const ISLAND_NW: Tex = t(33);
pub const ISLAND_NE: Tex = t(34);
pub const ISLAND_SW: Tex = t(35);
pub const ISLAND_SE: Tex = t(36);
pub const LAKE_NW: Tex = t(37);
pub const LAKE_NE: Tex = t(38);
pub const LAKE_SW: Tex = t(39);
pub const LAKE_SE: Tex = t(40);
pub const MISC_COBBLE_0: Tex = t(41);
pub const MISC_COBBLE_1: Tex = t(42);
pub const MISC_COBBLE_2: Tex = t(43);
pub const MISC_COBBLE_3: Tex = t(44);
pub const MISC_COBBLE_4: Tex = t(45);
pub const MISC_COBBLE_5: Tex = t(46);
pub const MISC_COBBLE_6: Tex = t(47);
pub const MISC_COBBLE_7: Tex = t(48);
pub const MISC_COBBLE_8: Tex = t(49);
pub const HOUSE_0: Tex = t(50);
pub const ROOF_0: Tex = t(51);
pub const HOUSE_1: Tex = t(52);
pub const ROOF_1: Tex = t(53);
pub const HOUSE_2: Tex = t(54);
pub const ROOF_2: Tex = t(55);
pub const HOUSE_3: Tex = t(56);
pub const ROOF_3: Tex = t(57);
pub const BUSH_0: Tex = t(58);
pub const BUSH_FLOWERS_0: Tex = t(59);
pub const BUSH_1: Tex = t(60);
pub const BUSH_FLOWERS_1: Tex = t(61);
pub const BUSH_2: Tex = t(62);
pub const BUSH_FLOWERS_2: Tex = t(63);
pub const ARTIFACT_NW: Tex = t(64);
pub const ARTIFACT_NE: Tex = t(65);
pub const ARTIFACT_SW: Tex = t(66);
pub const ARTIFACT_SE: Tex = t(67);
pub const ARTIFACT_GLYPH_NW: Tex = t(68);
pub const ARTIFACT_GLYPH_NE: Tex = t(69);
pub const ARTIFACT_GLYPH_SW: Tex = t(70);
pub const ARTIFACT_GLYPH_SE: Tex = t(71);
pub const GAME_PIECE_NW: Tex = t(72);
pub const GAME_PIECE_NE: Tex = t(73);
pub const GAME_PIECE_SW: Tex = t(74);
pub const GAME_PIECE_SE: Tex = t(75);
pub const HIGHLIGHT_NW: Tex = t(76);
pub const HIGHLIGHT_NE: Tex = t(77);
pub const HIGHLIGHT_SW: Tex = t(78);
pub const HIGHLIGHT_SE: Tex = t(79);
pub const GAME_PIECE_N: Tex = t(80);
pub const GAME_PIECE_S: Tex = t(81);
pub const GAME_PIECE_GRASS_0_SW: Tex = t(82);
pub const GAME_PIECE_GRASS_0_SE: Tex = t(83);
pub const GAME_PIECE_GRASS_1_SW: Tex = t(84);
pub const GAME_PIECE_GRASS_1_SE: Tex = t(85);
pub const GAME_PIECE_GRASS_2_SW: Tex = t(86);
pub const GAME_PIECE_GRASS_2_SE: Tex = t(87);
pub const GAME_PIECE_GRASS_3_SW: Tex = t(88);
pub const GAME_PIECE_GRASS_3_SE: Tex = t(89);
pub const GAME_PIECE_CRACKS_0_SW: Tex = t(90);
pub const GAME_PIECE_CRACKS_0_SE: Tex = t(91);
pub const GAME_PIECE_CRACKS_1_NW: Tex = t(92);
pub const GAME_PIECE_CRACKS_1_NE: Tex = t(93);
pub const GAME_PIECE_CRACKS_1_SE: Tex = t(94);
pub const GAME_PIECE_CRACKS_2_NE: Tex = t(95);
pub const GAME_PIECE_CRACKS_2_SE: Tex = t(96);
pub const GAME_PIECE_CRACKS_3_NW: Tex = t(97);
pub const GAME_PIECE_CRACKS_3_SW: Tex = t(98);
pub const GAME_PIECE_CRACKS_4_NW: Tex = t(99);
pub const GAME_PIECE_RUBBLE_0_NW: Tex = t(100);
pub const GAME_PIECE_RUBBLE_0_NE: Tex = t(101);
pub const GAME_PIECE_RUBBLE_0_SW: Tex = t(102);
pub const GAME_PIECE_RUBBLE_0_SE: Tex = t(103);
pub const GAME_PIECE_RUBBLE_1_NW: Tex = t(104);
pub const GAME_PIECE_RUBBLE_1_NE: Tex = t(105);
pub const GAME_PIECE_RUBBLE_1_SW: Tex = t(106);
pub const GAME_PIECE_RUBBLE_1_SE: Tex = t(107);
pub const GAME_PIECE_RUBBLE_2_NW: Tex = t(108);
pub const GAME_PIECE_RUBBLE_2_NE: Tex = t(109);
pub const GAME_PIECE_RUBBLE_2_SW: Tex = t(110);
pub const GAME_PIECE_RUBBLE_2_SE: Tex = t(111);
pub const SELECTION_SPINNER_1_NW: Tex = t(112);
pub const SELECTION_SPINNER_1_NE: Tex = t(113);
pub const SELECTION_SPINNER_1_SW: Tex = t(114);
pub const SELECTION_SPINNER_1_SE: Tex = t(115);
pub const SELECTION_SPINNER_2_NW: Tex = t(116);
pub const SELECTION_SPINNER_2_NE: Tex = t(117);
pub const SELECTION_SPINNER_2_SW: Tex = t(118);
pub const SELECTION_SPINNER_2_SE: Tex = t(119);
pub const SELECTION_SPINNER_3_NW: Tex = t(120);
pub const SELECTION_SPINNER_3_NE: Tex = t(121);
pub const SELECTION_SPINNER_3_SW: Tex = t(122);
pub const SELECTION_SPINNER_3_SE: Tex = t(123);
pub const SELECTION_SPINNER_4_NW: Tex = t(124);
pub const SELECTION_SPINNER_4_NE: Tex = t(125);
pub const SELECTION_SPINNER_4_SW: Tex = t(126);
pub const SELECTION_SPINNER_4_SE: Tex = t(127);
pub const DIALOG_NW: Tex = t(128);
pub const DIALOG_N: Tex = t(129);
pub const DIALOG_NE: Tex = t(130);
pub const DIALOG_W: Tex = t(131);
pub const DIALOG_CENTER: Tex = t(132);
pub const DIALOG_E: Tex = t(133);
pub const DIALOG_SW: Tex = t(134);
pub const DIALOG_S: Tex = t(135);
pub const DIALOG_SE: Tex = t(136);
pub const INFO_BUTTON_NW: Tex = t(137);
pub const INFO_BUTTON_NE: Tex = t(138);
pub const INFO_BUTTON_SW: Tex = t(139);
pub const INFO_BUTTON_SE: Tex = t(140);
pub const CLOSE_BUTTON_NW: Tex = t(141);
pub const CLOSE_BUTTON_NE: Tex = t(142);
pub const CLOSE_BUTTON_SW: Tex = t(143);
pub const CLOSE_BUTTON_SE: Tex = t(144);
pub const BUTTON_NOTIFICATION_SE: Tex = t(145);
pub const BARE_CLOSE_BUTTON_NW: Tex = t(146);
pub const BARE_CLOSE_BUTTON_NE: Tex = t(147);
pub const BARE_CLOSE_BUTTON_SW: Tex = t(148);
pub const BARE_CLOSE_BUTTON_SE: Tex = t(149);
pub const RESIGN_BUTTON_NW: Tex = t(150);
pub const RESIGN_BUTTON_NE: Tex = t(151);
pub const RESIGN_BUTTON_SW: Tex = t(152);
pub const RESIGN_BUTTON_SE: Tex = t(153);
pub const TRI_EAST_BUTTON_NW: Tex = t(154);
pub const TRI_EAST_BUTTON_NE: Tex = t(155);
pub const TRI_EAST_BUTTON_SW: Tex = t(156);
pub const TRI_EAST_BUTTON_SE: Tex = t(157);
pub const SKIP_NEXT_BUTTON_NW: Tex = t(158);
pub const SKIP_NEXT_BUTTON_NE: Tex = t(159);
pub const SKIP_NEXT_BUTTON_SW: Tex = t(160);
pub const SKIP_NEXT_BUTTON_SE: Tex = t(161);
pub const TRI_NORTH_BUTTON_NW: Tex = t(162);
pub const TRI_NORTH_BUTTON_NE: Tex = t(163);
pub const TRI_NORTH_BUTTON_SW: Tex = t(164);
pub const TRI_NORTH_BUTTON_SE: Tex = t(165);
pub const TRI_SOUTH_BUTTON_NW: Tex = t(166);
pub const TRI_SOUTH_BUTTON_NE: Tex = t(167);
pub const TRI_SOUTH_BUTTON_SW: Tex = t(168);
pub const TRI_SOUTH_BUTTON_SE: Tex = t(169);
pub const SKIP_PREV_BUTTON_NW: Tex = t(170);
pub const SKIP_PREV_BUTTON_NE: Tex = t(171);
pub const SKIP_PREV_BUTTON_SW: Tex = t(172);
pub const SKIP_PREV_BUTTON_SE: Tex = t(173);
pub const DICT_BUTTON_NW: Tex = t(174);
pub const DICT_BUTTON_NE: Tex = t(175);
pub const DICT_BUTTON_SW: Tex = t(176);
pub const DICT_BUTTON_SE: Tex = t(177);
pub const TOWN_BUTTON_NW: Tex = t(178);
pub const TOWN_BUTTON_NE: Tex = t(179);
pub const TOWN_BUTTON_SW: Tex = t(180);
pub const TOWN_BUTTON_SE: Tex = t(181);
pub const TOWN_BUTTON_ROOF_NW: Tex = t(182);
pub const TOWN_BUTTON_ROOF_NE: Tex = t(183);
pub const ARTIFACT_BUTTON_NW: Tex = t(184);
pub const ARTIFACT_BUTTON_NE: Tex = t(185);
pub const ARTIFACT_BUTTON_SW: Tex = t(186);
pub const ARTIFACT_BUTTON_SE: Tex = t(187);
pub const ARTIFACT_BUTTON_GLYPH_NW: Tex = t(188);
pub const ARTIFACT_BUTTON_GLYPH_NE: Tex = t(189);
pub const ARTIFACT_BUTTON_GLYPH_SW: Tex = t(190);
pub const ARTIFACT_BUTTON_GLYPH_SE: Tex = t(191);
pub const TERRAIN_BUTTON_NW: Tex = t(192);
pub const TERRAIN_BUTTON_NE: Tex = t(193);
pub const TERRAIN_BUTTON_SW: Tex = t(194);
pub const TERRAIN_BUTTON_SE: Tex = t(195);
pub const LETTER_SOUTH_A_NW: Tex = t(196);
pub const LETTER_SOUTH_A_NE: Tex = t(197);
pub const LETTER_SOUTH_A_SW: Tex = t(198);
pub const LETTER_SOUTH_A_SE: Tex = t(199);
pub const LETTER_SOUTH_B_NW: Tex = t(200);
pub const LETTER_SOUTH_B_NE: Tex = t(201);
pub const LETTER_SOUTH_B_SW: Tex = t(202);
pub const LETTER_SOUTH_B_SE: Tex = t(203);
pub const LETTER_SOUTH_C_NW: Tex = t(204);
pub const LETTER_SOUTH_C_NE: Tex = t(205);
pub const LETTER_SOUTH_C_SW: Tex = t(206);
pub const LETTER_SOUTH_C_SE: Tex = t(207);
pub const LETTER_SOUTH_D_NW: Tex = t(208);
pub const LETTER_SOUTH_D_NE: Tex = t(209);
pub const LETTER_SOUTH_D_SW: Tex = t(210);
pub const LETTER_SOUTH_D_SE: Tex = t(211);
pub const LETTER_SOUTH_E_NW: Tex = t(212);
pub const LETTER_SOUTH_E_NE: Tex = t(213);
pub const LETTER_SOUTH_E_SW: Tex = t(214);
pub const LETTER_SOUTH_E_SE: Tex = t(215);
pub const LETTER_SOUTH_F_NW: Tex = t(216);
pub const LETTER_SOUTH_F_NE: Tex = t(217);
pub const LETTER_SOUTH_F_SW: Tex = t(218);
pub const LETTER_SOUTH_F_SE: Tex = t(219);
pub const LETTER_SOUTH_G_NW: Tex = t(220);
pub const LETTER_SOUTH_G_NE: Tex = t(221);
pub const LETTER_SOUTH_G_SW: Tex = t(222);
pub const LETTER_SOUTH_G_SE: Tex = t(223);
pub const LETTER_SOUTH_H_NW: Tex = t(224);
pub const LETTER_SOUTH_H_NE: Tex = t(225);
pub const LETTER_SOUTH_H_SW: Tex = t(226);
pub const LETTER_SOUTH_H_SE: Tex = t(227);
pub const LETTER_SOUTH_I_NW: Tex = t(228);
pub const LETTER_SOUTH_I_NE: Tex = t(229);
pub const LETTER_SOUTH_I_SW: Tex = t(230);
pub const LETTER_SOUTH_I_SE: Tex = t(231);
pub const LETTER_SOUTH_J_NW: Tex = t(232);
pub const LETTER_SOUTH_J_NE: Tex = t(233);
pub const LETTER_SOUTH_J_SW: Tex = t(234);
pub const LETTER_SOUTH_J_SE: Tex = t(235);
pub const LETTER_SOUTH_K_NW: Tex = t(236);
pub const LETTER_SOUTH_K_NE: Tex = t(237);
pub const LETTER_SOUTH_K_SW: Tex = t(238);
pub const LETTER_SOUTH_K_SE: Tex = t(239);
pub const LETTER_SOUTH_L_NW: Tex = t(240);
pub const LETTER_SOUTH_L_NE: Tex = t(241);
pub const LETTER_SOUTH_L_SW: Tex = t(242);
pub const LETTER_SOUTH_L_SE: Tex = t(243);
pub const LETTER_SOUTH_M_NW: Tex = t(244);
pub const LETTER_SOUTH_M_NE: Tex = t(245);
pub const LETTER_SOUTH_M_SW: Tex = t(246);
pub const LETTER_SOUTH_M_SE: Tex = t(247);
pub const LETTER_SOUTH_N_NW: Tex = t(248);
pub const LETTER_SOUTH_N_NE: Tex = t(249);
pub const LETTER_SOUTH_N_SW: Tex = t(250);
pub const LETTER_SOUTH_N_SE: Tex = t(251);
pub const LETTER_SOUTH_O_NW: Tex = t(252);
pub const LETTER_SOUTH_O_NE: Tex = t(253);
pub const LETTER_SOUTH_O_SW: Tex = t(254);
pub const LETTER_SOUTH_O_SE: Tex = t(255);
pub const LETTER_SOUTH_P_NW: Tex = t(256);
pub const LETTER_SOUTH_P_NE: Tex = t(257);
pub const LETTER_SOUTH_P_SW: Tex = t(258);
pub const LETTER_SOUTH_P_SE: Tex = t(259);
pub const LETTER_SOUTH_Q_NW: Tex = t(260);
pub const LETTER_SOUTH_Q_NE: Tex = t(261);
pub const LETTER_SOUTH_Q_SW: Tex = t(262);
pub const LETTER_SOUTH_Q_SE: Tex = t(263);
pub const LETTER_SOUTH_R_NW: Tex = t(264);
pub const LETTER_SOUTH_R_NE: Tex = t(265);
pub const LETTER_SOUTH_R_SW: Tex = t(266);
pub const LETTER_SOUTH_R_SE: Tex = t(267);
pub const LETTER_SOUTH_S_NW: Tex = t(268);
pub const LETTER_SOUTH_S_NE: Tex = t(269);
pub const LETTER_SOUTH_S_SW: Tex = t(270);
pub const LETTER_SOUTH_S_SE: Tex = t(271);
pub const LETTER_SOUTH_T_NW: Tex = t(272);
pub const LETTER_SOUTH_T_NE: Tex = t(273);
pub const LETTER_SOUTH_T_SW: Tex = t(274);
pub const LETTER_SOUTH_T_SE: Tex = t(275);
pub const LETTER_SOUTH_U_NW: Tex = t(276);
pub const LETTER_SOUTH_U_NE: Tex = t(277);
pub const LETTER_SOUTH_U_SW: Tex = t(278);
pub const LETTER_SOUTH_U_SE: Tex = t(279);
pub const LETTER_SOUTH_V_NW: Tex = t(280);
pub const LETTER_SOUTH_V_NE: Tex = t(281);
pub const LETTER_SOUTH_V_SW: Tex = t(282);
pub const LETTER_SOUTH_V_SE: Tex = t(283);
pub const LETTER_SOUTH_W_NW: Tex = t(284);
pub const LETTER_SOUTH_W_NE: Tex = t(285);
pub const LETTER_SOUTH_W_SW: Tex = t(286);
pub const LETTER_SOUTH_W_SE: Tex = t(287);
pub const LETTER_SOUTH_X_NW: Tex = t(288);
pub const LETTER_SOUTH_X_NE: Tex = t(289);
pub const LETTER_SOUTH_X_SW: Tex = t(290);
pub const LETTER_SOUTH_X_SE: Tex = t(291);
pub const LETTER_SOUTH_Y_NW: Tex = t(292);
pub const LETTER_SOUTH_Y_NE: Tex = t(293);
pub const LETTER_SOUTH_Y_SW: Tex = t(294);
pub const LETTER_SOUTH_Y_SE: Tex = t(295);
pub const LETTER_SOUTH_Z_NW: Tex = t(296);
pub const LETTER_SOUTH_Z_NE: Tex = t(297);
pub const LETTER_SOUTH_Z_SW: Tex = t(298);
pub const LETTER_SOUTH_Z_SE: Tex = t(299);
pub const LETTER_NORTH_A_NW: Tex = t(300);
pub const LETTER_NORTH_A_NE: Tex = t(301);
pub const LETTER_NORTH_A_SW: Tex = t(302);
pub const LETTER_NORTH_A_SE: Tex = t(303);
pub const LETTER_NORTH_B_NW: Tex = t(304);
pub const LETTER_NORTH_B_NE: Tex = t(305);
pub const LETTER_NORTH_B_SW: Tex = t(306);
pub const LETTER_NORTH_B_SE: Tex = t(307);
pub const LETTER_NORTH_C_NW: Tex = t(308);
pub const LETTER_NORTH_C_NE: Tex = t(309);
pub const LETTER_NORTH_C_SW: Tex = t(310);
pub const LETTER_NORTH_C_SE: Tex = t(311);
pub const LETTER_NORTH_D_NW: Tex = t(312);
pub const LETTER_NORTH_D_NE: Tex = t(313);
pub const LETTER_NORTH_D_SW: Tex = t(314);
pub const LETTER_NORTH_D_SE: Tex = t(315);
pub const LETTER_NORTH_E_NW: Tex = t(316);
pub const LETTER_NORTH_E_NE: Tex = t(317);
pub const LETTER_NORTH_E_SW: Tex = t(318);
pub const LETTER_NORTH_E_SE: Tex = t(319);
pub const LETTER_NORTH_F_NW: Tex = t(320);
pub const LETTER_NORTH_F_NE: Tex = t(321);
pub const LETTER_NORTH_F_SW: Tex = t(322);
pub const LETTER_NORTH_F_SE: Tex = t(323);
pub const LETTER_NORTH_G_NW: Tex = t(324);
pub const LETTER_NORTH_G_NE: Tex = t(325);
pub const LETTER_NORTH_G_SW: Tex = t(326);
pub const LETTER_NORTH_G_SE: Tex = t(327);
pub const LETTER_NORTH_H_NW: Tex = t(328);
pub const LETTER_NORTH_H_NE: Tex = t(329);
pub const LETTER_NORTH_H_SW: Tex = t(330);
pub const LETTER_NORTH_H_SE: Tex = t(331);
pub const LETTER_NORTH_I_NW: Tex = t(332);
pub const LETTER_NORTH_I_NE: Tex = t(333);
pub const LETTER_NORTH_I_SW: Tex = t(334);
pub const LETTER_NORTH_I_SE: Tex = t(335);
pub const LETTER_NORTH_J_NW: Tex = t(336);
pub const LETTER_NORTH_J_NE: Tex = t(337);
pub const LETTER_NORTH_J_SW: Tex = t(338);
pub const LETTER_NORTH_J_SE: Tex = t(339);
pub const LETTER_NORTH_K_NW: Tex = t(340);
pub const LETTER_NORTH_K_NE: Tex = t(341);
pub const LETTER_NORTH_K_SW: Tex = t(342);
pub const LETTER_NORTH_K_SE: Tex = t(343);
pub const LETTER_NORTH_L_NW: Tex = t(344);
pub const LETTER_NORTH_L_NE: Tex = t(345);
pub const LETTER_NORTH_L_SW: Tex = t(346);
pub const LETTER_NORTH_L_SE: Tex = t(347);
pub const LETTER_NORTH_M_NW: Tex = t(348);
pub const LETTER_NORTH_M_NE: Tex = t(349);
pub const LETTER_NORTH_M_SW: Tex = t(350);
pub const LETTER_NORTH_M_SE: Tex = t(351);
pub const LETTER_NORTH_N_NW: Tex = t(352);
pub const LETTER_NORTH_N_NE: Tex = t(353);
pub const LETTER_NORTH_N_SW: Tex = t(354);
pub const LETTER_NORTH_N_SE: Tex = t(355);
pub const LETTER_NORTH_O_NW: Tex = t(356);
pub const LETTER_NORTH_O_NE: Tex = t(357);
pub const LETTER_NORTH_O_SW: Tex = t(358);
pub const LETTER_NORTH_O_SE: Tex = t(359);
pub const LETTER_NORTH_P_NW: Tex = t(360);
pub const LETTER_NORTH_P_NE: Tex = t(361);
pub const LETTER_NORTH_P_SW: Tex = t(362);
pub const LETTER_NORTH_P_SE: Tex = t(363);
pub const LETTER_NORTH_Q_NW: Tex = t(364);
pub const LETTER_NORTH_Q_NE: Tex = t(365);
pub const LETTER_NORTH_Q_SW: Tex = t(366);
pub const LETTER_NORTH_Q_SE: Tex = t(367);
pub const LETTER_NORTH_R_NW: Tex = t(368);
pub const LETTER_NORTH_R_NE: Tex = t(369);
pub const LETTER_NORTH_R_SW: Tex = t(370);
pub const LETTER_NORTH_R_SE: Tex = t(371);
pub const LETTER_NORTH_S_NW: Tex = t(372);
pub const LETTER_NORTH_S_NE: Tex = t(373);
pub const LETTER_NORTH_S_SW: Tex = t(374);
pub const LETTER_NORTH_S_SE: Tex = t(375);
pub const LETTER_NORTH_T_NW: Tex = t(376);
pub const LETTER_NORTH_T_NE: Tex = t(377);
pub const LETTER_NORTH_T_SW: Tex = t(378);
pub const LETTER_NORTH_T_SE: Tex = t(379);
pub const LETTER_NORTH_U_NW: Tex = t(380);
pub const LETTER_NORTH_U_NE: Tex = t(381);
pub const LETTER_NORTH_U_SW: Tex = t(382);
pub const LETTER_NORTH_U_SE: Tex = t(383);
pub const LETTER_NORTH_V_NW: Tex = t(384);
pub const LETTER_NORTH_V_NE: Tex = t(385);
pub const LETTER_NORTH_V_SW: Tex = t(386);
pub const LETTER_NORTH_V_SE: Tex = t(387);
pub const LETTER_NORTH_W_NW: Tex = t(388);
pub const LETTER_NORTH_W_NE: Tex = t(389);
pub const LETTER_NORTH_W_SW: Tex = t(390);
pub const LETTER_NORTH_W_SE: Tex = t(391);
pub const LETTER_NORTH_X_NW: Tex = t(392);
pub const LETTER_NORTH_X_NE: Tex = t(393);
pub const LETTER_NORTH_X_SW: Tex = t(394);
pub const LETTER_NORTH_X_SE: Tex = t(395);
pub const LETTER_NORTH_Y_NW: Tex = t(396);
pub const LETTER_NORTH_Y_NE: Tex = t(397);
pub const LETTER_NORTH_Y_SW: Tex = t(398);
pub const LETTER_NORTH_Y_SE: Tex = t(399);
pub const LETTER_NORTH_Z_NW: Tex = t(400);
pub const LETTER_NORTH_Z_NE: Tex = t(401);
pub const LETTER_NORTH_Z_SW: Tex = t(402);
pub const LETTER_NORTH_Z_SE: Tex = t(403);

pub mod quad {
    use super::*;

    pub const CHECKERBOARD: TexQuad = [CHECKERBOARD_NW, CHECKERBOARD_NE, CHECKERBOARD_SE, CHECKERBOARD_SW];
    pub const WATER_WITH_LAND: TexQuad = [WATER_WITH_LAND_NW, WATER_WITH_LAND_NE, WATER_WITH_LAND_SE, WATER_WITH_LAND_SW];
    pub const LAND_WITH_WATER: TexQuad = [LAND_WITH_WATER_NW, LAND_WITH_WATER_NE, LAND_WITH_WATER_SE, LAND_WITH_WATER_SW];
    pub const ISLAND: TexQuad = [ISLAND_NW, ISLAND_NE, ISLAND_SE, ISLAND_SW];
    pub const LAKE: TexQuad = [LAKE_NW, LAKE_NE, LAKE_SE, LAKE_SW];
    pub const ARTIFACT: TexQuad = [ARTIFACT_NW, ARTIFACT_NE, ARTIFACT_SE, ARTIFACT_SW];
    pub const ARTIFACT_GLYPH: TexQuad = [ARTIFACT_GLYPH_NW, ARTIFACT_GLYPH_NE, ARTIFACT_GLYPH_SE, ARTIFACT_GLYPH_SW];
    pub const GAME_PIECE: TexQuad = [GAME_PIECE_NW, GAME_PIECE_NE, GAME_PIECE_SE, GAME_PIECE_SW];
    pub const HIGHLIGHT: TexQuad = [HIGHLIGHT_NW, HIGHLIGHT_NE, HIGHLIGHT_SE, HIGHLIGHT_SW];
    pub const GAME_PIECE_GRASS_0: TexQuad = [NONE, NONE, GAME_PIECE_GRASS_0_SE, GAME_PIECE_GRASS_0_SW];
    pub const GAME_PIECE_GRASS_1: TexQuad = [NONE, NONE, GAME_PIECE_GRASS_1_SE, GAME_PIECE_GRASS_1_SW];
    pub const GAME_PIECE_GRASS_2: TexQuad = [NONE, NONE, GAME_PIECE_GRASS_2_SE, GAME_PIECE_GRASS_2_SW];
    pub const GAME_PIECE_GRASS_3: TexQuad = [NONE, NONE, GAME_PIECE_GRASS_3_SE, GAME_PIECE_GRASS_3_SW];
    pub const GAME_PIECE_CRACKS_0: TexQuad = [NONE, NONE, GAME_PIECE_CRACKS_0_SE, GAME_PIECE_CRACKS_0_SW];
    pub const GAME_PIECE_CRACKS_1: TexQuad = [GAME_PIECE_CRACKS_1_NW, GAME_PIECE_CRACKS_1_NE, GAME_PIECE_CRACKS_1_SE, NONE];
    pub const GAME_PIECE_CRACKS_2: TexQuad = [NONE, GAME_PIECE_CRACKS_2_NE, GAME_PIECE_CRACKS_2_SE, NONE];
    pub const GAME_PIECE_CRACKS_3: TexQuad = [GAME_PIECE_CRACKS_3_NW, NONE, NONE, GAME_PIECE_CRACKS_3_SW];
    pub const GAME_PIECE_CRACKS_4: TexQuad = [GAME_PIECE_CRACKS_4_NW, NONE, NONE, NONE];
    pub const GAME_PIECE_RUBBLE_0: TexQuad = [GAME_PIECE_RUBBLE_0_NW, GAME_PIECE_RUBBLE_0_NE, GAME_PIECE_RUBBLE_0_SE, GAME_PIECE_RUBBLE_0_SW];
    pub const GAME_PIECE_RUBBLE_1: TexQuad = [GAME_PIECE_RUBBLE_1_NW, GAME_PIECE_RUBBLE_1_NE, GAME_PIECE_RUBBLE_1_SE, GAME_PIECE_RUBBLE_1_SW];
    pub const GAME_PIECE_RUBBLE_2: TexQuad = [GAME_PIECE_RUBBLE_2_NW, GAME_PIECE_RUBBLE_2_NE, GAME_PIECE_RUBBLE_2_SE, GAME_PIECE_RUBBLE_2_SW];
    pub const SELECTION_SPINNER_1: TexQuad = [SELECTION_SPINNER_1_NW, SELECTION_SPINNER_1_NE, SELECTION_SPINNER_1_SE, SELECTION_SPINNER_1_SW];
    pub const SELECTION_SPINNER_2: TexQuad = [SELECTION_SPINNER_2_NW, SELECTION_SPINNER_2_NE, SELECTION_SPINNER_2_SE, SELECTION_SPINNER_2_SW];
    pub const SELECTION_SPINNER_3: TexQuad = [SELECTION_SPINNER_3_NW, SELECTION_SPINNER_3_NE, SELECTION_SPINNER_3_SE, SELECTION_SPINNER_3_SW];
    pub const SELECTION_SPINNER_4: TexQuad = [SELECTION_SPINNER_4_NW, SELECTION_SPINNER_4_NE, SELECTION_SPINNER_4_SE, SELECTION_SPINNER_4_SW];
    pub const DIALOG: TexQuad = [DIALOG_NW, DIALOG_NE, DIALOG_SE, DIALOG_SW];
    pub const INFO_BUTTON: TexQuad = [INFO_BUTTON_NW, INFO_BUTTON_NE, INFO_BUTTON_SE, INFO_BUTTON_SW];
    pub const CLOSE_BUTTON: TexQuad = [CLOSE_BUTTON_NW, CLOSE_BUTTON_NE, CLOSE_BUTTON_SE, CLOSE_BUTTON_SW];
    pub const BUTTON_NOTIFICATION: TexQuad = [NONE, NONE, BUTTON_NOTIFICATION_SE, NONE];
    pub const BARE_CLOSE_BUTTON: TexQuad = [BARE_CLOSE_BUTTON_NW, BARE_CLOSE_BUTTON_NE, BARE_CLOSE_BUTTON_SE, BARE_CLOSE_BUTTON_SW];
    pub const RESIGN_BUTTON: TexQuad = [RESIGN_BUTTON_NW, RESIGN_BUTTON_NE, RESIGN_BUTTON_SE, RESIGN_BUTTON_SW];
    pub const TRI_EAST_BUTTON: TexQuad = [TRI_EAST_BUTTON_NW, TRI_EAST_BUTTON_NE, TRI_EAST_BUTTON_SE, TRI_EAST_BUTTON_SW];
    pub const SKIP_NEXT_BUTTON: TexQuad = [SKIP_NEXT_BUTTON_NW, SKIP_NEXT_BUTTON_NE, SKIP_NEXT_BUTTON_SE, SKIP_NEXT_BUTTON_SW];
    pub const TRI_NORTH_BUTTON: TexQuad = [TRI_NORTH_BUTTON_NW, TRI_NORTH_BUTTON_NE, TRI_NORTH_BUTTON_SE, TRI_NORTH_BUTTON_SW];
    pub const TRI_SOUTH_BUTTON: TexQuad = [TRI_SOUTH_BUTTON_NW, TRI_SOUTH_BUTTON_NE, TRI_SOUTH_BUTTON_SE, TRI_SOUTH_BUTTON_SW];
    pub const SKIP_PREV_BUTTON: TexQuad = [SKIP_PREV_BUTTON_NW, SKIP_PREV_BUTTON_NE, SKIP_PREV_BUTTON_SE, SKIP_PREV_BUTTON_SW];
    pub const DICT_BUTTON: TexQuad = [DICT_BUTTON_NW, DICT_BUTTON_NE, DICT_BUTTON_SE, DICT_BUTTON_SW];
    pub const TOWN_BUTTON: TexQuad = [TOWN_BUTTON_NW, TOWN_BUTTON_NE, TOWN_BUTTON_SE, TOWN_BUTTON_SW];
    pub const TOWN_BUTTON_ROOF: TexQuad = [TOWN_BUTTON_ROOF_NW, TOWN_BUTTON_ROOF_NE, NONE, NONE];
    pub const ARTIFACT_BUTTON: TexQuad = [ARTIFACT_BUTTON_NW, ARTIFACT_BUTTON_NE, ARTIFACT_BUTTON_SE, ARTIFACT_BUTTON_SW];
    pub const ARTIFACT_BUTTON_GLYPH: TexQuad = [ARTIFACT_BUTTON_GLYPH_NW, ARTIFACT_BUTTON_GLYPH_NE, ARTIFACT_BUTTON_GLYPH_SE, ARTIFACT_BUTTON_GLYPH_SW];
    pub const TERRAIN_BUTTON: TexQuad = [TERRAIN_BUTTON_NW, TERRAIN_BUTTON_NE, TERRAIN_BUTTON_SE, TERRAIN_BUTTON_SW];
    pub const LETTER_SOUTH_A: TexQuad = [LETTER_SOUTH_A_NW, LETTER_SOUTH_A_NE, LETTER_SOUTH_A_SE, LETTER_SOUTH_A_SW];
    pub const LETTER_SOUTH_B: TexQuad = [LETTER_SOUTH_B_NW, LETTER_SOUTH_B_NE, LETTER_SOUTH_B_SE, LETTER_SOUTH_B_SW];
    pub const LETTER_SOUTH_C: TexQuad = [LETTER_SOUTH_C_NW, LETTER_SOUTH_C_NE, LETTER_SOUTH_C_SE, LETTER_SOUTH_C_SW];
    pub const LETTER_SOUTH_D: TexQuad = [LETTER_SOUTH_D_NW, LETTER_SOUTH_D_NE, LETTER_SOUTH_D_SE, LETTER_SOUTH_D_SW];
    pub const LETTER_SOUTH_E: TexQuad = [LETTER_SOUTH_E_NW, LETTER_SOUTH_E_NE, LETTER_SOUTH_E_SE, LETTER_SOUTH_E_SW];
    pub const LETTER_SOUTH_F: TexQuad = [LETTER_SOUTH_F_NW, LETTER_SOUTH_F_NE, LETTER_SOUTH_F_SE, LETTER_SOUTH_F_SW];
    pub const LETTER_SOUTH_G: TexQuad = [LETTER_SOUTH_G_NW, LETTER_SOUTH_G_NE, LETTER_SOUTH_G_SE, LETTER_SOUTH_G_SW];
    pub const LETTER_SOUTH_H: TexQuad = [LETTER_SOUTH_H_NW, LETTER_SOUTH_H_NE, LETTER_SOUTH_H_SE, LETTER_SOUTH_H_SW];
    pub const LETTER_SOUTH_I: TexQuad = [LETTER_SOUTH_I_NW, LETTER_SOUTH_I_NE, LETTER_SOUTH_I_SE, LETTER_SOUTH_I_SW];
    pub const LETTER_SOUTH_J: TexQuad = [LETTER_SOUTH_J_NW, LETTER_SOUTH_J_NE, LETTER_SOUTH_J_SE, LETTER_SOUTH_J_SW];
    pub const LETTER_SOUTH_K: TexQuad = [LETTER_SOUTH_K_NW, LETTER_SOUTH_K_NE, LETTER_SOUTH_K_SE, LETTER_SOUTH_K_SW];
    pub const LETTER_SOUTH_L: TexQuad = [LETTER_SOUTH_L_NW, LETTER_SOUTH_L_NE, LETTER_SOUTH_L_SE, LETTER_SOUTH_L_SW];
    pub const LETTER_SOUTH_M: TexQuad = [LETTER_SOUTH_M_NW, LETTER_SOUTH_M_NE, LETTER_SOUTH_M_SE, LETTER_SOUTH_M_SW];
    pub const LETTER_SOUTH_N: TexQuad = [LETTER_SOUTH_N_NW, LETTER_SOUTH_N_NE, LETTER_SOUTH_N_SE, LETTER_SOUTH_N_SW];
    pub const LETTER_SOUTH_O: TexQuad = [LETTER_SOUTH_O_NW, LETTER_SOUTH_O_NE, LETTER_SOUTH_O_SE, LETTER_SOUTH_O_SW];
    pub const LETTER_SOUTH_P: TexQuad = [LETTER_SOUTH_P_NW, LETTER_SOUTH_P_NE, LETTER_SOUTH_P_SE, LETTER_SOUTH_P_SW];
    pub const LETTER_SOUTH_Q: TexQuad = [LETTER_SOUTH_Q_NW, LETTER_SOUTH_Q_NE, LETTER_SOUTH_Q_SE, LETTER_SOUTH_Q_SW];
    pub const LETTER_SOUTH_R: TexQuad = [LETTER_SOUTH_R_NW, LETTER_SOUTH_R_NE, LETTER_SOUTH_R_SE, LETTER_SOUTH_R_SW];
    pub const LETTER_SOUTH_S: TexQuad = [LETTER_SOUTH_S_NW, LETTER_SOUTH_S_NE, LETTER_SOUTH_S_SE, LETTER_SOUTH_S_SW];
    pub const LETTER_SOUTH_T: TexQuad = [LETTER_SOUTH_T_NW, LETTER_SOUTH_T_NE, LETTER_SOUTH_T_SE, LETTER_SOUTH_T_SW];
    pub const LETTER_SOUTH_U: TexQuad = [LETTER_SOUTH_U_NW, LETTER_SOUTH_U_NE, LETTER_SOUTH_U_SE, LETTER_SOUTH_U_SW];
    pub const LETTER_SOUTH_V: TexQuad = [LETTER_SOUTH_V_NW, LETTER_SOUTH_V_NE, LETTER_SOUTH_V_SE, LETTER_SOUTH_V_SW];
    pub const LETTER_SOUTH_W: TexQuad = [LETTER_SOUTH_W_NW, LETTER_SOUTH_W_NE, LETTER_SOUTH_W_SE, LETTER_SOUTH_W_SW];
    pub const LETTER_SOUTH_X: TexQuad = [LETTER_SOUTH_X_NW, LETTER_SOUTH_X_NE, LETTER_SOUTH_X_SE, LETTER_SOUTH_X_SW];
    pub const LETTER_SOUTH_Y: TexQuad = [LETTER_SOUTH_Y_NW, LETTER_SOUTH_Y_NE, LETTER_SOUTH_Y_SE, LETTER_SOUTH_Y_SW];
    pub const LETTER_SOUTH_Z: TexQuad = [LETTER_SOUTH_Z_NW, LETTER_SOUTH_Z_NE, LETTER_SOUTH_Z_SE, LETTER_SOUTH_Z_SW];
    pub const LETTER_NORTH_A: TexQuad = [LETTER_NORTH_A_NW, LETTER_NORTH_A_NE, LETTER_NORTH_A_SE, LETTER_NORTH_A_SW];
    pub const LETTER_NORTH_B: TexQuad = [LETTER_NORTH_B_NW, LETTER_NORTH_B_NE, LETTER_NORTH_B_SE, LETTER_NORTH_B_SW];
    pub const LETTER_NORTH_C: TexQuad = [LETTER_NORTH_C_NW, LETTER_NORTH_C_NE, LETTER_NORTH_C_SE, LETTER_NORTH_C_SW];
    pub const LETTER_NORTH_D: TexQuad = [LETTER_NORTH_D_NW, LETTER_NORTH_D_NE, LETTER_NORTH_D_SE, LETTER_NORTH_D_SW];
    pub const LETTER_NORTH_E: TexQuad = [LETTER_NORTH_E_NW, LETTER_NORTH_E_NE, LETTER_NORTH_E_SE, LETTER_NORTH_E_SW];
    pub const LETTER_NORTH_F: TexQuad = [LETTER_NORTH_F_NW, LETTER_NORTH_F_NE, LETTER_NORTH_F_SE, LETTER_NORTH_F_SW];
    pub const LETTER_NORTH_G: TexQuad = [LETTER_NORTH_G_NW, LETTER_NORTH_G_NE, LETTER_NORTH_G_SE, LETTER_NORTH_G_SW];
    pub const LETTER_NORTH_H: TexQuad = [LETTER_NORTH_H_NW, LETTER_NORTH_H_NE, LETTER_NORTH_H_SE, LETTER_NORTH_H_SW];
    pub const LETTER_NORTH_I: TexQuad = [LETTER_NORTH_I_NW, LETTER_NORTH_I_NE, LETTER_NORTH_I_SE, LETTER_NORTH_I_SW];
    pub const LETTER_NORTH_J: TexQuad = [LETTER_NORTH_J_NW, LETTER_NORTH_J_NE, LETTER_NORTH_J_SE, LETTER_NORTH_J_SW];
    pub const LETTER_NORTH_K: TexQuad = [LETTER_NORTH_K_NW, LETTER_NORTH_K_NE, LETTER_NORTH_K_SE, LETTER_NORTH_K_SW];
    pub const LETTER_NORTH_L: TexQuad = [LETTER_NORTH_L_NW, LETTER_NORTH_L_NE, LETTER_NORTH_L_SE, LETTER_NORTH_L_SW];
    pub const LETTER_NORTH_M: TexQuad = [LETTER_NORTH_M_NW, LETTER_NORTH_M_NE, LETTER_NORTH_M_SE, LETTER_NORTH_M_SW];
    pub const LETTER_NORTH_N: TexQuad = [LETTER_NORTH_N_NW, LETTER_NORTH_N_NE, LETTER_NORTH_N_SE, LETTER_NORTH_N_SW];
    pub const LETTER_NORTH_O: TexQuad = [LETTER_NORTH_O_NW, LETTER_NORTH_O_NE, LETTER_NORTH_O_SE, LETTER_NORTH_O_SW];
    pub const LETTER_NORTH_P: TexQuad = [LETTER_NORTH_P_NW, LETTER_NORTH_P_NE, LETTER_NORTH_P_SE, LETTER_NORTH_P_SW];
    pub const LETTER_NORTH_Q: TexQuad = [LETTER_NORTH_Q_NW, LETTER_NORTH_Q_NE, LETTER_NORTH_Q_SE, LETTER_NORTH_Q_SW];
    pub const LETTER_NORTH_R: TexQuad = [LETTER_NORTH_R_NW, LETTER_NORTH_R_NE, LETTER_NORTH_R_SE, LETTER_NORTH_R_SW];
    pub const LETTER_NORTH_S: TexQuad = [LETTER_NORTH_S_NW, LETTER_NORTH_S_NE, LETTER_NORTH_S_SE, LETTER_NORTH_S_SW];
    pub const LETTER_NORTH_T: TexQuad = [LETTER_NORTH_T_NW, LETTER_NORTH_T_NE, LETTER_NORTH_T_SE, LETTER_NORTH_T_SW];
    pub const LETTER_NORTH_U: TexQuad = [LETTER_NORTH_U_NW, LETTER_NORTH_U_NE, LETTER_NORTH_U_SE, LETTER_NORTH_U_SW];
    pub const LETTER_NORTH_V: TexQuad = [LETTER_NORTH_V_NW, LETTER_NORTH_V_NE, LETTER_NORTH_V_SE, LETTER_NORTH_V_SW];
    pub const LETTER_NORTH_W: TexQuad = [LETTER_NORTH_W_NW, LETTER_NORTH_W_NE, LETTER_NORTH_W_SE, LETTER_NORTH_W_SW];
    pub const LETTER_NORTH_X: TexQuad = [LETTER_NORTH_X_NW, LETTER_NORTH_X_NE, LETTER_NORTH_X_SE, LETTER_NORTH_X_SW];
    pub const LETTER_NORTH_Y: TexQuad = [LETTER_NORTH_Y_NW, LETTER_NORTH_Y_NE, LETTER_NORTH_Y_SE, LETTER_NORTH_Y_SW];
    pub const LETTER_NORTH_Z: TexQuad = [LETTER_NORTH_Z_NW, LETTER_NORTH_Z_NE, LETTER_NORTH_Z_SE, LETTER_NORTH_Z_SW];
}