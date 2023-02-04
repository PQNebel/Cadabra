use super::*;

use PieceType::*;
use MoveType::*;

// Macros to expand const generics for move generation
macro_rules! generate_moves_match_has_enpassant {
    ($move_gen: expr, $pos: expr, $is_quiescence: expr, $is_white: expr, $is_sorting: expr) => {
        match $pos.enpassant_square.is_some() {
            true =>  $move_gen.generate_moves::<$is_quiescence, $is_white, $is_sorting, true>($pos),
            false => $move_gen.generate_moves::<$is_quiescence, $is_white, $is_sorting, false>($pos),
        }
    };
}
macro_rules! generate_moves_match_is_sorting {
    ($move_gen: expr, $pos: expr, $is_quiescence: expr, $is_white: expr, $is_sorting: expr) => {
        match $is_sorting {
            true =>  generate_moves_match_has_enpassant!($move_gen, $pos, $is_quiescence, $is_white, true),
            false => generate_moves_match_has_enpassant!($move_gen, $pos, $is_quiescence, $is_white, false),
        }
    };
}
macro_rules! generate_moves_match_color {
    ($move_gen: expr, $pos: expr, $is_quiescence: expr, $is_sorting: expr) => {
        match $pos.active_color {
            Color::White => generate_moves_match_is_sorting!($move_gen, $pos, $is_quiescence, true, $is_sorting),
            Color::Black => generate_moves_match_is_sorting!($move_gen, $pos, $is_quiescence, false, $is_sorting),
        }
    };
}
macro_rules! generate_moves_match_move_types {
    ($move_gen: expr, $pos: expr, $is_quiescence: expr, $is_sorting: expr) => {
        match $is_quiescence {
            true =>  generate_moves_match_color!($move_gen, $pos, true, $is_sorting),
            false => generate_moves_match_color!($move_gen, $pos, false, $is_sorting),
        }
    };
}
macro_rules! generate_moves {
    ($move_gen: expr, $pos: expr, $is_quiescence: expr, $is_sorting: expr) => {
        // Match color
        generate_moves_match_move_types!($move_gen, $pos, $is_quiescence, $is_sorting)
    };
}

pub struct MoveList {
    insert_index: usize,
    extract_index: usize,

    move_list: [Move; 100],
}

impl Iterator for MoveList {
    type Item = Move;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        if self.extract_index == self.insert_index {
            return None
        };

        let extracted = self.move_list[self.extract_index];
        self.extract_index += 1;
        Some(extracted)
    }
}

impl MoveList {
    /// Creates a new move list and populates it with all legal moves in the position
    pub fn new(position: &Position, is_quiescence: bool, sort: bool, pv_move: Option<Move>) -> Self {
        let mut list = Self {
            insert_index: 0,
            extract_index: 0,

            move_list: [Default::default(); 100], // Check if this is necessary
        };

        if let Some(pv) = pv_move {
            list.insert(pv);
        }

        generate_moves!(list, position, is_quiescence, sort);

        list
    }

    pub fn length(&self) -> usize {
        self.insert_index
    }

    /// Extracts the best move in the list
    #[inline(always)]
    pub fn extract_best(&mut self) -> Option<Move> {
        if self.extract_index == self.insert_index {
            return None
        };

        let mut best_index = self.extract_index;

        for i in self.extract_index..self.insert_index {
            let best_score = self.move_list[best_index].score;
            let score = self.move_list[i].score;

            if score > best_score {
                best_index = i
            }
        }

        self.move_list.swap(self.extract_index, best_index);

        let extracted = self.move_list[self.extract_index];
        self.extract_index += 1;

        Some(extracted)
    }

    /// Inserts the move into the list, and scores it if 
    #[inline(always)]
    fn insert_and_score(&mut self, new_move: &mut Move, is_sorting: bool, is_quiescence: bool) {
        if is_quiescence && !new_move.is_capture() {
            return
        }

        if is_sorting {
            Self::score_move(new_move) // Maybe handle scoring different
        }

        self.insert(*new_move)
    }

    #[inline(always)]
    fn insert(&mut self, new_move: Move) {
        self.move_list[self.insert_index] = new_move;
        self.insert_index += 1;
    }

    fn score_move(m: &mut Move) {
        m.score = 10; // Fake it. Should probably be moved to seperate place
    }

    /// Generate more legal moves for the position
    #[inline(always)]
    fn generate_moves<const IS_QUIESCENCE: bool, const IS_WHITE: bool, const IS_SORTING: bool, const HAS_ENPASSANT: bool>(&mut self, pos: &Position) {
        let active_color = if IS_WHITE { Color::White } else { Color::Black };

        let is_sorting = IS_SORTING;
        let has_enpassant = HAS_ENPASSANT;
        let is_quiescence = IS_QUIESCENCE;

        let opp_color = opposite_color(active_color);

        let attacked_squares = {
            pos.get_attacked(opp_color, Pawn).or(
            pos.get_attacked(opp_color, Knight)).or(
            pos.get_attacked(opp_color, Rook)).or(
            pos.get_attacked(opp_color, Bishop)).or(
            pos.get_attacked(opp_color, Queen)).or(
            pos.get_attacked(opp_color, King))
        };

        // Go straight to check evasions if in check
        {
            let check_mask = Self::get_check_mask(pos, active_color);
            let valid_square_and_checkmask = (pos.get_color_bitboard(active_color).not()).and(check_mask);

            let checkers = check_mask.and(pos.get_color_bitboard(opposite_color(active_color))).count();
            if !check_mask.not().is_empty() && checkers > 0 {
                if checkers == 1 {
                    // Single check, generate evasions
                    self.generate_pawn_moves(pos, active_color, is_sorting, false, has_enpassant, valid_square_and_checkmask);
                    self.generate_normal_moves(pos, active_color, is_sorting, Knight, valid_square_and_checkmask, false);
                    self.generate_normal_moves(pos, active_color, is_sorting, Bishop, valid_square_and_checkmask, false);
                    self.generate_normal_moves(pos, active_color, is_sorting, Rook, valid_square_and_checkmask, false);
                    self.generate_normal_moves(pos, active_color, is_sorting, Queen, valid_square_and_checkmask, false);
                }
                // Double check => only the king can move
                self.generate_king_moves(pos, active_color, attacked_squares, is_sorting, is_quiescence);
                return
            }
        }

        let empty_or_enemy = pos.get_color_bitboard(active_color).not();

        // Castling
        match active_color {
            Color::White => {
                if pos.castling_ability & (CastlingAbility::WhiteKingSide as u8) != 0 {
                    let mask = CastlingAbility::WhiteKingSide.mask().and(attacked_squares);
                    if mask.is_empty() {
                        self.insert_and_score(&mut Move::new_custom(60, 63, King, MoveType::CastleKingSide), is_sorting, is_quiescence)
                    }
                }
                if pos.castling_ability & (CastlingAbility::WhiteQueenSide as u8) != 0 {
                    let mask = CastlingAbility::WhiteQueenSide.mask().and(attacked_squares);
                    if mask.is_empty() {
                        self.insert_and_score(&mut Move::new_custom(60, 56, King, MoveType::CastleQueenSide), is_sorting, is_quiescence)
                    }
                }
            },
            Color::Black => {
                if pos.castling_ability & (CastlingAbility::BlackKingSide as u8) != 0 {
                    let mask = CastlingAbility::BlackKingSide.mask().and(attacked_squares);
                    if mask.is_empty() {
                        self.insert_and_score(&mut Move::new_custom(4, 7, King, MoveType::CastleKingSide), is_sorting, is_quiescence)
                    }
                }
                if pos.castling_ability & (CastlingAbility::WhiteQueenSide as u8) != 0 {
                    let mask = CastlingAbility::BlackQueenSide.mask().and(attacked_squares);
                    if mask.is_empty() {
                        self.insert_and_score(&mut Move::new_custom(4, 0, King, MoveType::CastleQueenSide), is_sorting, is_quiescence)
                    }
                }
            }
        }
        
        self.generate_pawn_moves(pos, active_color, is_sorting, is_quiescence, has_enpassant, Bitboard::new_full());
        self.generate_normal_moves(pos, active_color, is_sorting, Knight, empty_or_enemy, is_quiescence);
        self.generate_normal_moves(pos, active_color, is_sorting, Bishop, empty_or_enemy, is_quiescence);
        self.generate_normal_moves(pos, active_color, is_sorting, Rook, empty_or_enemy, is_quiescence);
        self.generate_normal_moves(pos, active_color, is_sorting, Queen, empty_or_enemy, is_quiescence);
        self.generate_king_moves(pos, active_color, attacked_squares, is_sorting, is_quiescence);
    }

    #[inline(always)]
    fn generate_pawn_moves(&mut self, pos: &Position, color: Color, is_sorting: bool, is_quiescence: bool, has_enpassant: bool, valid_mask: Bitboard) {
        let mut pawns = pos.get_bitboard(color, Pawn);
        while let Some(from_square) = pawns.extract_bit() {
            // Quiet target square
            let to_square = match color {
                Color::White => (from_square as i8 - 8) as u8,
                Color::Black => from_square + 8,
            };

            let raw_captures = get_pawn_attack_table(from_square, color);

            // Enpassant
            if has_enpassant {
                let enp_square = unsafe { pos.enpassant_square.unwrap_unchecked() };
                if raw_captures.get_bit(enp_square as u8) {
                    let captured_square = match color {
                        Color::White => to_square + 8,
                        Color::Black => to_square - 8,
                    };
                    self.insert_and_score(&mut Move::new_custom(from_square, enp_square as u8, Pawn, DoublePush(Square::from(captured_square))), is_sorting, false)
                }
            }

            let mut captures = raw_captures.and(pos.get_color_bitboard(opposite_color(color))).and(Self::get_pin_mask(pos, color, from_square, Pawn)).and(valid_mask);

            // Promotions
            if Bitboard::from(END_RANKS_MASK).get_bit(to_square) {
                // Regular promotion
                if !pos.all_occupancies.get_bit(to_square) {
                    self.insert_and_score(&mut Move::new_promotion(from_square, to_square, Queen,  false), is_sorting, false);
                    self.insert_and_score(&mut Move::new_promotion(from_square, to_square, Rook,   false), is_sorting, false);
                    self.insert_and_score(&mut Move::new_promotion(from_square, to_square, Bishop, false), is_sorting, false);
                    self.insert_and_score(&mut Move::new_promotion(from_square, to_square, Knight, false), is_sorting, false);
                }

                // Capture promotions
                while let Some(to_square) = captures.extract_bit() {
                    self.insert_and_score(&mut Move::new_promotion(from_square, to_square, Queen,  true), is_sorting, false);
                    self.insert_and_score(&mut Move::new_promotion(from_square, to_square, Rook,   true), is_sorting, false);
                    self.insert_and_score(&mut Move::new_promotion(from_square, to_square, Bishop, true), is_sorting, false);
                    self.insert_and_score(&mut Move::new_promotion(from_square, to_square, Knight, true), is_sorting, false);
                }

                return
            }

            // Regular captures
            while let Some(to_square) = captures.extract_bit() {
                self.insert_and_score(&mut Move::new_normal(from_square, to_square, Pawn, true), is_sorting, false)
            }

            // Quiet & double push
            if !is_quiescence && !pos.all_occupancies.get_bit(to_square) && valid_mask.get_bit(to_square) {
                // Normal move
                self.insert_and_score(&mut Move::new_normal(from_square, to_square, Pawn, false), is_sorting, false);

                // Double push
                // Only possible if pawn hasn't moved. Needs to come after promotions to not override them
                if Bitboard::from(PAWN_INIT_RANKS_MASK).get_bit(from_square) {
                    let double_push_square = match color {
                        Color::White => (from_square as i8 - 16) as u8,
                        Color::Black => from_square + 16,
                    };

                    if !pos.all_occupancies.get_bit(double_push_square) && valid_mask.get_bit(double_push_square) {
                        self.insert_and_score(&mut Move::new_custom(from_square, double_push_square, Pawn, DoublePush(Square::from(to_square))), is_sorting, false);
                    }
                }
            }
        }
    }

    #[inline(always)]
    fn generate_normal_moves(&mut self, pos: &Position, color: Color, is_sorting: bool, piece_type: PieceType, empty_or_enemy: Bitboard, is_quiescence: bool) {
        let mut pieces = pos.get_bitboard(color, piece_type);
        while let Some(from_square) = pieces.extract_bit() {
            let mut moves = get_attack_table(from_square, color, piece_type, pos.all_occupancies).and(empty_or_enemy).and(Self::get_pin_mask(pos, color, from_square, piece_type));
        
            while let Some(to_square) = moves.extract_bit() {
                let is_capture = pos.all_occupancies.get_bit(to_square);
                self.insert_and_score(&mut Move::new_normal(from_square, to_square, piece_type, is_capture), is_sorting, is_quiescence)
            }
        }
    }

    /// Generates all legal king moves
    #[inline(always)]
    fn generate_king_moves(&mut self, pos: &Position, color: Color, attacked_squares: Bitboard, is_sorting: bool, is_quiescence: bool) {
        let king_pos = pos.king_position(color);

        let mut pos = *pos;
        pos.remove_piece(color, King, king_pos);
        let opp_color = opposite_color(color);

        let mut legal_moves = get_king_attack_table(king_pos).and(attacked_squares.not()).and(pos.get_color_bitboard(color).not());
        
        while let Some(to_square) = legal_moves.extract_bit() {
            let is_capture = pos.all_occupancies.get_bit(to_square);
            self.insert_and_score(&mut Move::new_normal(king_pos, to_square, King, is_capture), is_sorting, is_quiescence)
        }
    }

    /// Should be delegated to pregenerated Constants for sliders
    #[inline(always)]
    pub fn get_pin_mask(pos: &Position, color: Color, square: u8, piece_type: PieceType) -> Bitboard {
        let mut pos = *pos;

        pos.remove_piece(color, piece_type, square);

        let mask = MoveList::get_check_mask(&pos, color);

        mask
    }

    #[inline(always)]
    /// Call with the color of the active player
    pub fn get_enpassant_pin_mask(pos: &Position, color: Color, square: u8) -> Bitboard {
        let mut pos = *pos;

        pos.remove_piece(opposite_color(color), Pawn, square);

        let mask = MoveList::get_check_mask(&pos, color);

        mask
    }

    /// Should be delegated to pregenerated Constants for sliders
    #[inline(always)]
    fn get_check_mask(pos: &Position, color: Color) -> Bitboard {
        let mut mask = Bitboard::new_blank();

        let opp_color = opposite_color(color);

        let king_pos = pos.king_position(color);

        // Leapers
        mask = mask.or(
            (get_pawn_attack_table(king_pos, opp_color).and(pos.get_bitboard(opp_color, Pawn))).or(
            get_knight_attack_table(king_pos).and(pos.get_bitboard(opp_color, Knight)))
        );

        // Hv Sliders
        {
            let opp_hv_sliders = pos.get_bitboard(opp_color, Rook).or(
                pos.get_bitboard(opp_color, Queen)
            );

            let king_file = Bitboard::from(FILE_MASKS[king_pos as usize]);
            let king_rank = Bitboard::from(RANK_MASKS[king_pos as usize]);

            let king_hv_rays = get_rook_attack_table(king_pos, pos.all_occupancies);

            let mut sliders = opp_hv_sliders;
            while let Some(slider) = sliders.extract_bit() {
                let mut slider_board = Bitboard::new_blank();
                slider_board.set_bit(slider);
                let slider_rays = (get_rook_attack_table(slider, pos.all_occupancies).and(pos.all_occupancies.not())).or(slider_board);

                let slider_hori = slider_rays.and(Bitboard::from(RANK_MASKS[slider as usize]));
                let king_hori = king_hv_rays.and(king_rank);
                mask = mask.or(king_hori.and(slider_hori));

                let slider_vert = slider_rays.and(Bitboard::from(FILE_MASKS[slider as usize]));
                let king_vert = king_hv_rays.and(king_file);
                mask = mask.or(king_vert.and(slider_vert));
            }
        }
        
        // Diagonal Sliders
        {
            let opp_diag_sliders = pos.get_bitboard(opp_color, Bishop).or(
                pos.get_bitboard(opp_color, Queen)
            );

            let king_diag1 = Bitboard::from(DIAG1_MASKS[king_pos as usize]);
            let king_diag2 = Bitboard::from(DIAG2_MASKS[king_pos as usize]);

            let king_diag_rays = get_bishop_attack_table(king_pos, pos.all_occupancies);

            let mut sliders = opp_diag_sliders;
            while let Some(slider) = sliders.extract_bit() {
                let mut slider_board = Bitboard::new_blank();
                slider_board.set_bit(slider);
                let slider_rays = (get_bishop_attack_table(slider, pos.all_occupancies)).and(pos.all_occupancies.not()).or(slider_board);

                let slider_hori = slider_rays.and(Bitboard::from(DIAG1_MASKS[slider as usize]));
                let king_hori = king_diag_rays.and(king_diag1);
                mask = mask.or(king_hori.and(slider_hori));

                let slider_vert = slider_rays.and(Bitboard::from(DIAG2_MASKS[slider as usize]));
                let king_vert = king_diag_rays.and(king_diag2);
                mask = mask.or(king_vert.and(slider_vert));
            }
        }

        if mask.is_empty() {
            Bitboard::new_full()
        } else {
            mask
        }
    }
}

#[test]
pub fn test() {
    // let mut pos = Position::new_from_start_pos();
    // let mut pos = Position::new_from_fen("k7/8/4r3/3R4/3K1N2/8/4b3/8 w - - 0 1").unwrap();
    let mut pos = Position::new_from_fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1 ").unwrap();

    let moves = MoveList::new(&mut pos, false, false, None).collect::<Vec<Move>>();

    println!("Moves: {}", moves.len());
    println!("Captures: {}", moves.iter().filter(|m| m.is_capture()).count());
    println!("E.p.: {}", moves.iter().filter(|m| if let EnpassantCapture(_) = m.move_type { true } else { false }).count());
    println!("Castles: {}", moves.iter().filter(|m| m.move_type == CastleKingSide || m.move_type == CastleQueenSide).count());
    println!("Promotions: {}", moves.iter().filter(|m| if let Promotion(_) = m.move_type { true } else { false } || if let CapturePromotion(_) = m.move_type { true } else { false }).count());

    println!();
    
    for m in moves {
        println!("{}", m)
    }
}