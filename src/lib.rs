use std::collections::HashMap;

use Piece::*;
use rand::{Rng, thread_rng};
use lazy_static::lazy_static;
use GameResult::*;

#[derive(Copy,Clone, Debug, PartialEq, Eq)]
enum Piece {
    King = 0, Queen = 1, Bishop= 2, Knight = 3, Rook = 4, Pawn = 5, Void
}

#[derive(Copy,Clone, Debug, PartialEq, Eq)]
pub enum GameResult {
    WhiteWin,
    BlackWin,
    Draw
}

const PIECES: [Piece; 6] = [Pawn, Knight, Bishop, Rook, Queen, King];

const PROMOTIONS: [Piece; 4] = [Queen, Rook, Knight, Bishop];

const SQUARE_NAME: [&str; 64] = [//this is also the order of the squares used throughout the engine
        "h1", "g1", "f1", "e1", "d1", "c1", "b1", "a1",
        "h2", "g2", "f2", "e2", "d2", "c2", "b2", "a2",
        "h3", "g3", "f3", "e3", "d3", "c3", "b3", "a3",
        "h4", "g4", "f4", "e4", "d4", "c4", "b4", "a4",
        "h5", "g5", "f5", "e5", "d5", "c5", "b5", "a5",
        "h6", "g6", "f6", "e6", "d6", "c6", "b6", "a6",
        "h7", "g7", "f7", "e7", "d7", "c7", "b7", "a7",
        "h8", "g8", "f8", "e8", "d8", "c8", "b8", "a8",
    ];


#[derive(Debug, Clone)]
struct Move {
    from: u64,
    destination: u64,
    piece: Piece,
    promotion: Piece
}


//use startpos() or from_fen() to create a new position
#[derive(Clone)]
pub struct Position {
    w_board: [u64; 6],
    b_board: [u64; 6],
    w_all: u64,
    b_all: u64,
    w_turn: bool, //true if white; false if black
    en_passent_target_square: u64,
    //castling_rights: [bool; 4], //white kingside, white queenside, black kingside, blackqueenside
    legal_moves: Vec<Move>,
}

impl Position {

    //returns the default chess position
    pub fn startpos() -> Position {
        Position::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
    }

    fn empty() -> Position {
        Position { w_board: [0; 6], w_all: 0, b_board: [0; 6], b_all: 0, w_turn: true, en_passent_target_square: 0, legal_moves: vec![]}
    }

    //parses a fen string to a chess position
    pub fn from_fen(fen_string: &str) -> Position {
        let mut ptr = 0b1u64 << 63; //start at a8
        let mut position = Position::empty();

        let separator = fen_string.chars().position(|c| c == ' ').unwrap();
        let (fen_board, flags) = (&fen_string[..separator], &fen_string[separator..]);

        for byte in fen_board.as_bytes().iter() {
            if byte <= &b'9' && byte >= &b'1' {
                ptr >>= byte-b'0';
                continue;
            }
            match byte {
                b'r' => {position.b_board[Rook as usize] |= ptr},
                b'b' => {position.b_board[Bishop as usize] |= ptr},
                b'n' => {position.b_board[Knight as usize] |= ptr},
                b'q' => {position.b_board[Queen as usize] |= ptr},
                b'k' => {position.b_board[King as usize] |= ptr},
                b'p' => {position.b_board[Pawn as usize] |= ptr},
                b'R' => {position.w_board[Rook as usize] |= ptr},
                b'B' => {position.w_board[Bishop as usize] |= ptr},
                b'N' => {position.w_board[Knight as usize] |= ptr},
                b'Q' => {position.w_board[Queen as usize] |= ptr},
                b'K' => {position.w_board[King as usize] |= ptr},
                b'P' => {position.w_board[Pawn as usize] |= ptr},
                _ => {} 
            }
            if byte != &b'/' {ptr >>= 1;}
        }
        for byte in flags.as_bytes().iter() {
            if byte == &b'w' {
                position.w_turn = true;
            }
            else if byte == &b'b' {
                position.w_turn = false;
            }
        }
        for piece in PIECES {
            position.w_all |= position.w_board[piece as usize];
            position.b_all |= position.b_board[piece as usize];
        }
        position.calculate_legal_moves();
        position
    }

    //checks whether or not there are any legal moves, if there
    //are no legal moves the game is over, use get_result() to get the result
    pub fn game_in_progress(&self) -> bool {
        return self.legal_moves.len() != 0
    }

    //returns the result of the game, should only be used
    //after game_in_progress returns false
    pub fn get_result(&self) -> GameResult {
        let blocker_board = self.w_all | self.b_all;
        if self.w_turn {
            let king_pos = self.w_board[King as usize].trailing_zeros() as usize;
            if square_attacked_by_black(self.clone(), blocker_board, king_pos) {
                return BlackWin
            }
        }
        else {
            let king_pos = self.b_board[King as usize].trailing_zeros() as usize;
            if square_attacked_by_white(self.clone(), blocker_board, king_pos) {
                return WhiteWin
            }
        }
        Draw
    }

    //returns all legal moves in standard uci format
    pub fn get_legal_moves(&mut self) -> Vec<String> {//should only be used for human interaction
        let mut bitboard_square_to_name: HashMap<u64, &str> = HashMap::new();
        for square in 0..64 {
            let bitboard_square = 0b1u64 << square;
            let square_name = SQUARE_NAME[square];
            bitboard_square_to_name.insert(bitboard_square, square_name);
        }
        
        let mut moves: Vec<String> = vec![];
        for m in &self.legal_moves {
            if m.promotion == Void {
                moves.push(bitboard_square_to_name[&m.from].to_string() + bitboard_square_to_name[&m.destination]);
            }
            else {
                let piece: &str = match m.promotion {
                    Queen => "q",
                    Rook => "r",
                    Knight => "n",
                    Bishop => "b",
                    _ => {""}
                };
                moves.push(bitboard_square_to_name[&m.from].to_string() + bitboard_square_to_name[&m.destination]+piece);
            }
            
        }
        moves
    }


    //returns all legal moves in standard uci format from a given square
    pub fn get_square_legal_moves(&mut self, square: &str) -> Vec<String> {//should only be used for human interaction
        let mut square_legal_moves: Vec<String> = vec![];
        let legal_moves = self.get_legal_moves();
        for m in legal_moves {
            if &m[..2] == square {
                square_legal_moves.push(m[2..].to_string());
            }
        }
        square_legal_moves
    }

    fn calculate_legal_moves(&mut self) {
        self.legal_moves.clear();
        let blocker_board = self.w_all | self.b_all; 
        let mut king_pos: usize = 0;
        for square in 0..64 {
            let bitboard_square = 0b1u64 << square;
            if self.w_turn {
                match self.get_w_piece(bitboard_square) {
                    Queen => {self.add_w_queen_moves(square, blocker_board, bitboard_square)},
                    Rook => {self.add_w_rook_moves(square, blocker_board, bitboard_square)},
                    Bishop => {self.add_w_bishop_moves(square, blocker_board, bitboard_square)},
                    Knight => {self.add_w_knight_moves(square, bitboard_square)},
                    Pawn => {self.add_w_pawn_moves(square, blocker_board ,bitboard_square)},
                    King => {self.add_w_king_moves(square, bitboard_square); king_pos = square},
                    Void => {}
                } 
            }
            else {
                match self.get_b_piece(bitboard_square) {
                    Queen => {self.add_b_queen_moves(square, blocker_board, bitboard_square)},
                    Rook => {self.add_b_rook_moves(square, blocker_board, bitboard_square)},
                    Bishop => {self.add_b_bishop_moves(square, blocker_board, bitboard_square)},
                    Knight => {self.add_b_knight_moves(square, bitboard_square)},
                    Pawn => {self.add_b_pawn_moves(square, blocker_board, bitboard_square)},
                    King => {self.add_b_king_moves(square, bitboard_square); king_pos = square},
                    Void => {}
                } 
            }
            
        }
        //this removes all moves that leaves the king in check, code is messy and hard to debug and not very fast
        //so it should definetly be replaced with a proper pinned pieces bitboard implementation
        let mut moves = self.legal_moves.clone();
        if self.w_turn {
            moves.retain(|m | self.w_king_capture_filter(m.clone(), king_pos));
        } else {
            moves.retain(|m | self.b_king_capture_filter(m.clone(), king_pos));
        }
        self.legal_moves = moves;
    }

    //plays a move from standard uci format, does not check if the move is legal
    //to check for legality, first use get_legal_moves and check if the move is in the vec
    //uci example "e2e4"  move the piece from e2 to e4
    //promotions in uci are handled by adding a letter after the move q => Queen, r => Rook, n => Knight, b => Bishop
    //example a7a8q    move the peice from a7 to a8 and promote to a Queen
    //this function can handle castling even tought castling is not yet implemented for get_legal_moves
    //so to castle simply make the move in standard uci format, nothing will break
    pub fn make_move(&mut self, m: &str) { //should only be used for human interaction
        let mut name_to_bitboard_square: HashMap<&str, u64> = HashMap::new();
        for square in 0..64 {
            let bitboard_square = 0b1u64 << square;
            let square_name = SQUARE_NAME[square];
            name_to_bitboard_square.insert(square_name, bitboard_square);
        }
        let from = name_to_bitboard_square[&m[..2]];
        let destination = name_to_bitboard_square[&m[2..4]];
        let promotion_piece = &m[4..];
        let promotion = match  promotion_piece {
            "q" => Queen,
            "r" => Rook,
            "b" => Bishop,
            "n" => Knight,
            _ => Void
            
        };

        let piece = if self.w_turn {self.get_w_piece(from)} else {self.get_b_piece(from)};
        if self.w_turn {
            self.make_w_move(Move{from, destination, piece, promotion: promotion});
        }
        else {
            self.make_b_move(Move{from, destination, piece, promotion: promotion});
        }
        
        ;
    }

    fn make_w_move(&mut self, m: Move) {
        //moving the piece
        self.w_board[m.piece as usize] ^= m.from | m.destination;
        self.w_all ^= m.from | m.destination;

        //clearing captured pieces
        if self.b_all & m.destination != 0 {
            self.b_all &= !m.destination;
            for piece in PIECES {
                self.b_board[piece as usize] &= !m.destination;
            }
        }

        if m.promotion != Void {
            self.w_board[m.piece as usize] &= !m.destination;
            self.w_board[m.promotion as usize] |= m.destination
        }

        else if m.piece == King && (m.from >> 2) == m.destination {
            self.w_board[Rook as usize] ^= (m.destination >> 1) | (m.destination << 1);
            self.w_all ^= (m.destination >> 1) | (m.destination << 1);
        }
        else if m.piece == King && (m.from << 2) == m.destination {
            self.w_board[Rook as usize] ^= (m.destination >> 1) | (m.destination << 2);
            self.w_all ^= (m.destination >> 1) | (m.destination << 2);
        }

        else if m.piece == Pawn && m.destination == self.en_passent_target_square {
            self.b_all &= !(m.destination >> 8);
            self.b_board[Pawn as usize] &= !(m.destination >>8);
            
        }
        self.en_passent_target_square = 0;
        if m.piece == Pawn && (m.from << 16) == m.destination {
            self.en_passent_target_square = m.from << 8;
        }
        
        self.w_turn = false;
        self.legal_moves.clear();
        self.calculate_legal_moves();
    }

    fn make_b_move(&mut self, m: Move) {
        //moving the piece
        self.b_board[m.piece as usize] ^= m.from | m.destination;
        self.b_all ^= m.from | m.destination;

        //clearing captured pieces
        if self.w_all & m.destination != 0 {
            self.w_all &= !m.destination;
            for piece in PIECES {
                self.w_board[piece as usize] &= !m.destination;
            }
        }

        if m.promotion != Void {
            self.b_board[m.piece as usize] &= !m.destination;
            self.b_board[m.promotion as usize] |= m.destination
        }

        else if m.piece == King && (m.from >> 2) == m.destination {
            self.w_board[Rook as usize] ^= (m.destination >> 1) | (m.destination << 1);
            self.w_all ^= (m.destination >> 1) | (m.destination << 1);
        }
        else if m.piece == King && (m.from << 2) == m.destination {
            self.w_board[Rook as usize] ^= (m.destination >> 1) | (m.destination << 2);
            self.w_all ^= (m.destination >> 1) | (m.destination << 2);
        }

        else if m.piece == Pawn && m.destination == self.en_passent_target_square {
            self.w_all &= !(m.destination << 8);
            self.w_board[Pawn as usize] &= !(m.destination <<8);
        }
        self.en_passent_target_square = 0;
        if m.piece == Pawn && (m.from >> 16) == m.destination {
            self.en_passent_target_square = m.from >> 8;
        }
        self.w_turn = true;
        self.legal_moves.clear();
        self.calculate_legal_moves();
    }

    fn w_king_capture_filter(&mut self, m: Move, king_pos: usize) -> bool {
        let mut king_pos_copy = king_pos;
        let mut pos_clone = self.clone();
        
        //moving the piece
        pos_clone.w_board[m.piece as usize] ^= m.from | m.destination;
        pos_clone.w_all ^= m.from | m.destination;

        //clearing captured pieces
        if pos_clone.b_all & m.destination != 0 {
            pos_clone.b_all &= !m.destination;
            for piece in PIECES {
                pos_clone.b_board[piece as usize] &= !m.destination;
            }
        }

        if m.piece == Pawn && m.destination == pos_clone.en_passent_target_square {
            pos_clone.b_all &= !(m.destination >> 8);
            pos_clone.b_board[Pawn as usize] &= !(m.destination >>8);
        }

        //if the king is moved we need to update its position
        if m.piece == King {
            king_pos_copy = m.destination.trailing_zeros() as usize;
        }
  
        let blocker_board = pos_clone.w_all | pos_clone.b_all;
        return !square_attacked_by_black(pos_clone ,blocker_board, king_pos_copy);
    }

    fn b_king_capture_filter(&mut self, m: Move, king_pos: usize) -> bool {
        let mut king_pos_copy = king_pos;
        let mut pos_clone = self.clone();
        
        //moving the piece
        pos_clone.b_board[m.piece as usize] ^= m.from | m.destination;
        pos_clone.b_all ^= m.from | m.destination;

        //clearing captured pieces
        if pos_clone.w_all & m.destination != 0 {
            pos_clone.w_all &= !m.destination;
            for piece in PIECES {
                pos_clone.w_board[piece as usize] &= !m.destination;
            }
        }

        if m.piece == Pawn && m.destination == pos_clone.en_passent_target_square {
            pos_clone.w_all &= !(m.destination << 8);
            pos_clone.w_board[Pawn as usize] &= !(m.destination <<8);
        }


        //if the king is moved we need to update its position
        if m.piece == King {
            king_pos_copy = m.destination.trailing_zeros() as usize;
        }

        let blocker_board = pos_clone.w_all | pos_clone.b_all;
        return !square_attacked_by_white(pos_clone ,blocker_board, king_pos_copy);
    }

    

    

    fn get_w_piece(&self, bitboard_square: u64) -> Piece {
        for piece in PIECES {
            if self.w_board[piece as usize] & bitboard_square == bitboard_square {
                return piece
            }
        }
        Void
    }

    fn get_b_piece(&self, bitboard_square: u64) -> Piece {
        for piece in PIECES {
            if self.b_board[piece as usize] & bitboard_square == bitboard_square {
                return piece
            }
        }
        Void
    }
    

    fn add_w_pawn_moves(&mut self, square: usize, blocker_board: u64  ,bitboard_square: u64){  
        let mut legal_moves = W_PAWN_FORWARD_MASK[square] & !(blocker_board) |
        W_PAWN_DOUBLEFORWARD_MASK[square] & !(blocker_board | blocker_board << 8) |
        (W_PAWN_CAPTURE_MASK[square] & (self.b_all | self.en_passent_target_square));
        self.add_moves(&mut legal_moves, bitboard_square, Pawn);
    }

    fn add_w_king_moves(&mut self, square: usize, bitboard_square: u64){
        let mut legal_moves = KING_MASK[square] & !self.w_all;
        self.add_moves(&mut legal_moves, bitboard_square, King);
    }

    fn add_w_knight_moves(&mut self, square: usize, bitboard_square: u64){
        let mut legal_moves = KNIGHT_MASK[square] & !self.w_all;
        self.add_moves(&mut legal_moves, bitboard_square, Knight);
    }

    fn add_w_bishop_moves(&mut self, square: usize, blocker_board: u64, bitboard_square: u64){
        let bishop_blocker_board = blocker_board & BISHOP_BLOCKER_MASK[square];
        let (magic_number, magic_lookup) = &BISHOP_MAGIC_MASK[square];
        let mut legal_moves = magic_lookup[(bishop_blocker_board.wrapping_mul(*magic_number) >> BISHOP_MAGIC_SHIFT[square]) as usize]
         & !self.w_all;
        self.add_moves(&mut legal_moves, bitboard_square, Bishop);
    }

    fn add_w_rook_moves(&mut self, square: usize, blocker_board: u64, bitboard_square: u64){
        let rook_blocker_board = blocker_board & ROOK_BLOCKER_MASK[square];
        let (magic_number, magic_lookup) = unsafe{&ROOK_MAGIC_MASK[square]};
        let mut legal_moves = magic_lookup[(rook_blocker_board.wrapping_mul(*magic_number) >> ROOK_MAGIC_SHIFT[square]) as usize]
         & !self.w_all;
        self.add_moves(&mut legal_moves, bitboard_square, Rook);
    }

    fn add_w_queen_moves(&mut self, square: usize, blocker_board: u64, bitboard_square: u64){
        let rook_blocker_board = blocker_board & ROOK_BLOCKER_MASK[square];
        let (magic_number, magic_lookup) = unsafe{&ROOK_MAGIC_MASK[square]};
        let legal_rook_moves = magic_lookup[(rook_blocker_board.wrapping_mul(*magic_number) >> ROOK_MAGIC_SHIFT[square]) as usize]
         & !self.w_all;
        let bishop_blocker_board = blocker_board & BISHOP_BLOCKER_MASK[square];
        let (magic_number, magic_lookup) = &BISHOP_MAGIC_MASK[square];
        let legal_bishop_moves = magic_lookup[(bishop_blocker_board.wrapping_mul(*magic_number) >> BISHOP_MAGIC_SHIFT[square]) as usize]
          & !self.w_all;

        let mut legal_moves = legal_rook_moves | legal_bishop_moves;

        self.add_moves(&mut legal_moves, bitboard_square, Queen);
    }

    fn add_b_pawn_moves(&mut self, square: usize, blocker_board: u64  ,bitboard_square: u64){  
        let mut legal_moves = B_PAWN_FORWARD_MASK[square] & !(blocker_board) |
        B_PAWN_DOUBLEFORWARD_MASK[square] & !(blocker_board | blocker_board >> 8) |
        (B_PAWN_CAPTURE_MASK[square] & (self.w_all | self.en_passent_target_square));
        self.add_moves(&mut legal_moves, bitboard_square, Pawn);
    }

    fn add_b_king_moves(&mut self, square: usize, bitboard_square: u64){
        let mut legal_moves = KING_MASK[square] & !self.b_all;
        self.add_moves(&mut legal_moves, bitboard_square, King);
    }

    fn add_b_knight_moves(&mut self, square: usize, bitboard_square: u64){
        let mut legal_moves = KNIGHT_MASK[square] & !self.b_all;
        self.add_moves(&mut legal_moves, bitboard_square, Knight);
    }

    fn add_b_bishop_moves(&mut self, square: usize, blocker_board: u64, bitboard_square: u64){
        let bishop_blocker_board = blocker_board & BISHOP_BLOCKER_MASK[square];
        let (magic_number, magic_lookup) = &BISHOP_MAGIC_MASK[square];
        let mut legal_moves = magic_lookup[(bishop_blocker_board.wrapping_mul(*magic_number) >> BISHOP_MAGIC_SHIFT[square]) as usize]
         & !self.b_all;
        self.add_moves(&mut legal_moves, bitboard_square, Bishop);
    }

    fn add_b_rook_moves(&mut self, square: usize, blocker_board: u64, bitboard_square: u64){
        let rook_blocker_board = blocker_board & ROOK_BLOCKER_MASK[square];
        let (magic_number, magic_lookup) = unsafe{&ROOK_MAGIC_MASK[square]};
        let mut legal_moves = magic_lookup[(rook_blocker_board.wrapping_mul(*magic_number) >> ROOK_MAGIC_SHIFT[square]) as usize]
         & !self.b_all;
        self.add_moves(&mut legal_moves, bitboard_square, Rook);
    }

    fn add_b_queen_moves(&mut self, square: usize, blocker_board: u64, bitboard_square: u64){
        let rook_blocker_board = blocker_board & ROOK_BLOCKER_MASK[square];
        let (magic_number, magic_lookup) = unsafe{&ROOK_MAGIC_MASK[square]};
        let legal_rook_moves = magic_lookup[(rook_blocker_board.wrapping_mul(*magic_number) >> ROOK_MAGIC_SHIFT[square]) as usize]
         & !self.b_all;
        let bishop_blocker_board = blocker_board & BISHOP_BLOCKER_MASK[square];
        let (magic_number, magic_lookup) = &BISHOP_MAGIC_MASK[square];
        let legal_bishop_moves = magic_lookup[(bishop_blocker_board.wrapping_mul(*magic_number) >> BISHOP_MAGIC_SHIFT[square]) as usize]
          & !self.b_all;

        let mut legal_moves = legal_rook_moves | legal_bishop_moves;

        self.add_moves(&mut legal_moves, bitboard_square, Queen);
    }

    fn add_moves(&mut self, bitboard: &mut u64, from: u64, piece: Piece) {
            while *bitboard != 0 {
                let last_bit: u64 = *bitboard & !(*bitboard -1); //getting the last bit
                *bitboard &= *bitboard -1; //removing the last bit
                if piece == Pawn && last_bit & (RANK[0] | RANK[7]) != 0 { //checking if move is a promotion
                    for promotion in PROMOTIONS {
                        self.legal_moves.push(Move {from: from, destination: last_bit, piece: piece, promotion: promotion}); 
                    }
                }
                else {
                    self.legal_moves.push(Move {from: from, destination: last_bit, piece: piece, promotion: Void}); 
                }
                
            }
    }

    //prints the board using chess unicode chars
    pub fn print(&self) {
        let mut rank = 8;
        for mut square in 0..64 as u8 {
            if square % 8 == 0 {
                print!("\n{} ", rank);
                rank -= 1;
            }
            square = 63 - square;
            let bitboard_square = 0b1u64 << square;
            match self.get_w_piece(bitboard_square) {
                Queen => {print!("♕"); continue;},
                Rook => {print!("♖"); continue;},
                Bishop => {print!("♗"); continue;},
                Knight => {print!("♘"); continue;},
                Pawn => {print!("♙"); continue;},
                King => {print!("♔"); continue;},
                Void => {}
            };
            match self.get_b_piece(bitboard_square) {
                Queen => {print!("♛")},
                Rook => {print!("♜")},
                Bishop => {print!("♝")},
                Knight => {print!("♞")},
                Pawn => {print!("♟︎")},
                King => {print!("♚")},
                Void => {print!("_")}
            };
        }
        println!("\n  ABCDEFGH");
    }

}


fn square_attacked_by_black(position: Position, blocker_board: u64, square: usize) -> bool {
    if W_PAWN_CAPTURE_MASK[square] & position.b_board[Pawn as usize] != 0 {
        return true
    }
    if KING_MASK[square] & position.b_board[King as usize] != 0 {
        return true
    }
    if KNIGHT_MASK[square] & position.b_board[Knight as usize] != 0 {
        return true
    }
    let bishop_blocker_board = blocker_board & BISHOP_BLOCKER_MASK[square];
    let (magic_number, magic_lookup) = BISHOP_MAGIC_MASK[square];
    let magic_index = bishop_blocker_board.wrapping_mul(magic_number) >> BISHOP_MAGIC_SHIFT[square];
    if magic_lookup[magic_index as usize] & (position.b_board[Bishop as usize] | position.b_board[Queen as usize]) != 0 {
        return true
    }
    let rook_blocker_board = blocker_board & ROOK_BLOCKER_MASK[square];
    let (magic_number, magic_lookup) = unsafe{ROOK_MAGIC_MASK[square]};
    let magic_index = rook_blocker_board.wrapping_mul(magic_number) >> ROOK_MAGIC_SHIFT[square];
    if magic_lookup[magic_index as usize] & (position.b_board[Rook as usize] | position.b_board[Queen as usize]) != 0 {
        return true
    }
    false

}

fn square_attacked_by_white(position: Position, blocker_board: u64, square: usize) -> bool {
    if B_PAWN_CAPTURE_MASK[square] & position.w_board[Pawn as usize] != 0 {
        return true
    }
    if KING_MASK[square] & position.w_board[King as usize] != 0 {
        return true
    }
    if KNIGHT_MASK[square] & position.w_board[Knight as usize] != 0 {
        return true
    }
    let bishop_blocker_board = blocker_board & BISHOP_BLOCKER_MASK[square];
    let (magic_number, magic_lookup) = BISHOP_MAGIC_MASK[square];
    let magic_index = bishop_blocker_board.wrapping_mul(magic_number) >> BISHOP_MAGIC_SHIFT[square];
    if magic_lookup[magic_index as usize] & (position.w_board[Bishop as usize] | position.w_board[Queen as usize]) != 0 {
        return true
    }
    let rook_blocker_board = blocker_board & ROOK_BLOCKER_MASK[square];
    let (magic_number, magic_lookup) = unsafe{ROOK_MAGIC_MASK[square]};
    let magic_index = rook_blocker_board.wrapping_mul(magic_number) >> ROOK_MAGIC_SHIFT[square];
    if magic_lookup[magic_index as usize] & (position.w_board[Rook as usize] | position.w_board[Queen as usize]) != 0 {
        return true
    }
    false

}


const NOT_ON_H_FILE: u64 = 0b1111111011111110111111101111111011111110111111101111111011111110u64;
const NOT_ON_A_FILE: u64 = 0b0111111101111111011111110111111101111111011111110111111101111111u64;

const NOT_ON_GH_FILE: u64 = 0b1111110011111100111111001111110011111100111111001111110011111100u64;
const NOT_ON_AB_FILE: u64 = 0b0011111100111111001111110011111100111111001111110011111100111111u64;


fn w_pawn_forward_mask(bitboard_square: u64) -> u64 {
    bitboard_square << 8
}

lazy_static! {
    static ref W_PAWN_FORWARD_MASK: [u64; 64] = {
        let mut mask: [u64; 64] = [0; 64];
        for square in 0..64 {
            let bitboard_square = 0b1u64 << square;
            mask[square] = w_pawn_forward_mask(bitboard_square);
        }
        mask
    };
}

fn w_pawn_doubleforward_mask(bitboard_square: u64) -> u64 {
    (bitboard_square & (RANK[1])) << 16
}

lazy_static! {
    static ref W_PAWN_DOUBLEFORWARD_MASK: [u64; 64] = {
        let mut mask: [u64; 64] = [0; 64];
        for square in 0..64 {
            let bitboard_square = 0b1u64 << square;
            mask[square] = w_pawn_doubleforward_mask(bitboard_square);
        }
        mask
    };
}

fn w_pawn_capture_mask(bitboard_square: u64) -> u64 {
    ((bitboard_square & NOT_ON_H_FILE) << 7) | ((bitboard_square & NOT_ON_A_FILE) << 9)
}

lazy_static! {
    static ref W_PAWN_CAPTURE_MASK: [u64; 64] = {
        let mut mask: [u64; 64] = [0; 64];
        for square in 0..64 {
            let bitboard_square = 0b1u64 << square;
            mask[square] = w_pawn_capture_mask(bitboard_square);
        }
        mask
    };
}   

fn b_pawn_forward_mask(bitboard_square: u64) -> u64 {
    bitboard_square >> 8
}

lazy_static! {
    static ref B_PAWN_FORWARD_MASK: [u64; 64] = {
        let mut mask: [u64; 64] = [0; 64];
        for square in 0..64 {
            let bitboard_square = 0b1u64 << square;
            mask[square] = b_pawn_forward_mask(bitboard_square);
        }
        mask
    };
}

fn b_pawn_doubleforward_mask(bitboard_square: u64) -> u64 {
    (bitboard_square & (RANK[6])) >> 16
}

lazy_static! {
    static ref B_PAWN_DOUBLEFORWARD_MASK: [u64; 64] = {
        let mut mask: [u64; 64] = [0; 64];
        for square in 0..64 {
            let bitboard_square = 0b1u64 << square;
            mask[square] = b_pawn_doubleforward_mask(bitboard_square);
        }
        mask
    };
}

fn b_pawn_capture_mask(bitboard_square: u64) -> u64 {
    ((bitboard_square & NOT_ON_H_FILE) >> 9) | ((bitboard_square & NOT_ON_A_FILE) >> 7)
}

lazy_static! {
    static ref B_PAWN_CAPTURE_MASK: [u64; 64] = {
        let mut mask: [u64; 64] = [0; 64];
        for square in 0..64 {
            let bitboard_square = 0b1u64 << square;
            mask[square] = b_pawn_capture_mask(bitboard_square);
        }
        mask
    };
}  

fn king_mask(bitboard_square: u64) -> u64 {
    ((bitboard_square & NOT_ON_H_FILE) << 7) | ((bitboard_square & NOT_ON_H_FILE) >> 1) | ((bitboard_square & NOT_ON_H_FILE) >> 9) |
    (bitboard_square << 8) | (bitboard_square >> 8) |
    ((bitboard_square & NOT_ON_A_FILE) << 9) | ((bitboard_square & NOT_ON_A_FILE) << 1) | ((bitboard_square & NOT_ON_A_FILE) >> 7)
}

lazy_static! {
    static ref KING_MASK: [u64; 64] = {
        let mut mask: [u64; 64] = [0; 64];
        for square in 0..64 {
            let bitboard_square = 0b1u64 << square;
            mask[square] = king_mask(bitboard_square);
        }
        mask
    };
}  

fn knight_mask(bitboard_square: u64) -> u64 {
    ((bitboard_square & NOT_ON_A_FILE) << 17) | ((bitboard_square & NOT_ON_A_FILE) >> 15) |
    ((bitboard_square & NOT_ON_H_FILE) << 15) | ((bitboard_square & NOT_ON_H_FILE) >> 17) |
    ((bitboard_square & NOT_ON_AB_FILE) << 10) | ((bitboard_square & NOT_ON_AB_FILE) >> 6) |
    ((bitboard_square & NOT_ON_GH_FILE) << 6) | ((bitboard_square & NOT_ON_GH_FILE) >> 10)
}

lazy_static! {
    static ref KNIGHT_MASK: [u64; 64] = {
        let mut mask: [u64; 64] = [0; 64];
        for square in 0..64 {
            let bitboard_square = 0b1u64 << square;
            mask[square] = knight_mask(bitboard_square);
        }
        mask
    };
}  

//fn print_board(board: u64) {
//    for mut square in 0..64 as u8 {
//        if square % 8 == 0 {
//            print!("\n")
//        }
//        square = 63 - square;
//        
//        let bitboard = 0b1u64 << square;
//        if board & bitboard == bitboard {
//            print!("1")
//        }
//        else {
//            print!(".")
//        }
//    }
//    println!();
//}


const ROOK_MAGIC_SHIFT: [u8; 64]=[
	52, 53, 53, 53, 53, 53, 53, 52,
	53, 54, 54, 54, 54, 54, 54, 53,
	53, 54, 54, 54, 54, 54, 54, 53,
	53, 54, 54, 54, 54, 54, 54, 53,
	53, 54, 54, 54, 54, 54, 54, 53,
	53, 54, 54, 54, 54, 54, 54, 53,
	53, 54, 54, 54, 54, 54, 54, 53,
	52, 53, 53, 53, 53, 53, 53, 52
];

const FILE: [u64; 8] = [
    0b1000000010000000100000001000000010000000100000001000000010000000u64,
    0b0100000001000000010000000100000001000000010000000100000001000000u64,
    0b0010000000100000001000000010000000100000001000000010000000100000u64,
    0b0001000000010000000100000001000000010000000100000001000000010000u64,
    0b0000100000001000000010000000100000001000000010000000100000001000u64,
    0b0000010000000100000001000000010000000100000001000000010000000100u64,
    0b0000001000000010000000100000001000000010000000100000001000000010u64,
    0b0000000100000001000000010000000100000001000000010000000100000001u64,
];

const RANK: [u64; 8] = [
    0b11111111u64,
    0b11111111u64 << 8,
    0b11111111u64 << 16,
    0b11111111u64 << 24,
    0b11111111u64 << 32,
    0b11111111u64 << 40,
    0b11111111u64 << 48,
    0b11111111u64 << 56,
];


fn rook_mask(bitboard_square: u64, blocker_board: u64) -> u64 {
    let mut mask = 0b0u64;
    let mut ptr = bitboard_square;
    //up
    while ptr & RANK[7] == 0{
        ptr <<= 8;
        mask |= ptr;
        if ptr & blocker_board != 0 {break}
    }
    ptr = bitboard_square;
    //down
    while ptr & RANK[0] == 0{
        ptr >>= 8;
        mask |= ptr;
        if ptr & blocker_board != 0 {break}
    }
    ptr = bitboard_square;
    //left
    while ptr & FILE[0] == 0{
        ptr <<= 1;
        mask |= ptr;
        if ptr & blocker_board != 0 {break}
    }
    ptr = bitboard_square;
    //right
    while ptr & FILE[7] == 0{
        ptr >>= 1;
        mask |= ptr;
        if ptr & blocker_board != 0 {break}
    }
    mask    
}

fn rook_all_blockers_mask(square: u8) -> u64{ 
    let file = 7 - (square % 8); 
    let rank = square / 8;
    let not_on_ah = !(FILE[0] | FILE[7]);
    let not_on_18 = !(RANK[0] | RANK[7]);
    ((FILE[file as usize] & not_on_18) ^ (RANK[rank as usize] & not_on_ah)) & !(0b1u64 << square)
}

lazy_static! {
    static ref ROOK_BLOCKER_MASK: [u64; 64] = {
        let mut mask: [u64; 64] = [0; 64];
        for square in 0..64 {
            mask[square] = rook_all_blockers_mask(square as u8);
        }
        mask
    };
}  

const BISHOP_MAGIC_SHIFT: [u8; 64] = [
	58, 59, 59, 59, 59, 59, 59, 58,
	59, 59, 59, 59, 59, 59, 59, 59,
	59, 59, 57, 57, 57, 57, 59, 59,
	59, 59, 57, 55, 55, 57, 59, 59,
	59, 59, 57, 55, 55, 57, 59, 59,
	59, 59, 57, 57, 57, 57, 59, 59,
	59, 59, 59, 59, 59, 59, 59, 59,
	58, 59, 59, 59, 59, 59, 59, 58
];

fn bishop_mask(bitboard_square: u64, blocker_board: u64) -> u64 {
    let mut mask = 0b0u64;
    let mut ptr = bitboard_square;
    //up right
    while ptr & (RANK[7] | FILE[7]) == 0{
        ptr <<= 7;
        mask |= ptr;
        if ptr & blocker_board != 0 {break}
    }
    ptr = bitboard_square;
    //down right
    while ptr & (RANK[0]| FILE[7]) == 0{
        ptr >>= 9;
        mask |= ptr;
        if ptr & blocker_board != 0 {break}
    }
    ptr = bitboard_square;
    //up left
    while ptr & (RANK[7] | FILE[0]) == 0{
        ptr <<= 9;
        mask |= ptr;
        if ptr & blocker_board != 0 {break}
    }
    ptr = bitboard_square;
    //down left
    while ptr & (RANK[0] | FILE[0]) == 0{
        ptr >>= 7;
        mask |= ptr;
        if ptr & blocker_board != 0 {break}
    }
    mask    
}

fn bishop_all_blockers_mask(square: u8) -> u64{ 
    let bitboard_edges = FILE[0] | FILE[7] | RANK[0] | RANK[7];
    bishop_mask(0b1u64 << square, 0b0u64) & !bitboard_edges
}

lazy_static! {
    static ref BISHOP_BLOCKER_MASK: [u64; 64] = {
        let mut mask: [u64; 64] = [0; 64];
        for square in 0..64 {
            mask[square] = bishop_all_blockers_mask(square as u8);
        }
        mask
    };
}  

fn find_magic(piece: Piece, square: u8) -> (u64, Vec<u64>) {
    let mut count = 0;
    //looping through random numbers until a magic number is found
    loop {
        //the random u64 with ands is chosen since move magic number have a small amount of 1s
        let maybe_magic = thread_rng().gen::<u64>() & thread_rng().gen::<u64>() & thread_rng().gen::<u64>(); 
        let result: Option<Vec<u64>> = check_if_magic(piece, square, maybe_magic);
        match result {
            Some(lookup) => {
                //the number is magic!
                println!("found magic after {} attempts", count);
                return (maybe_magic, lookup)
            },
            None => {count += 1}
        }
    }
}

fn check_if_magic(piece: Piece, square: u8, magic_candidate: u64) -> Option<Vec<u64>> {
   
    let mut lookup: Vec<u64> = 
    if piece == Rook {vec![0; 1 << (64-ROOK_MAGIC_SHIFT[square as usize])]} 
    else {vec![0; 1 << (64-BISHOP_MAGIC_SHIFT[square as usize])]};

    let all_blockers_set = 
    if piece == Rook {rook_all_blockers_mask(square)}
    else {bishop_all_blockers_mask(square)};

    let mut blocker_subset: u64 = 0;

    //Carry-Rippler trick to enumerate all subsets in a set
    //https://www.chessprogramming.org/Traversing_Subsets_of_a_Set#All_Subsets_of_any_Set
    //the set is a mask containing all possible blocking squares
    //so the subsets will be all possible configurations of blocker boards
    loop {
    let move_mask = 
    if piece == Rook {rook_mask(0b1u64 << square, blocker_subset)}
    else {bishop_mask(0b1u64 << square, blocker_subset)};

    //a magic index is the blocker board for the square multiplied with a magic number and then shifted by the amount of relevant blocker squares
    //magic index = (blocker*magic number)>>(magic bitshift); 
    //move mask = lookup table [magic index];
    //https://www.chessprogramming.org/Magic_Bitboards
    //this is how the move mask later can be accessed from the lookup table
    let magic_index =  
    if piece == Rook {blocker_subset.wrapping_mul(magic_candidate) >> ROOK_MAGIC_SHIFT[square as usize]}
    else {blocker_subset.wrapping_mul(magic_candidate) >> BISHOP_MAGIC_SHIFT[square as usize]};

    if lookup[magic_index as usize] == 0 {
        lookup[magic_index as usize] = move_mask;
    }
    else if lookup[magic_index as usize] == move_mask{
        //good hash collision
    }
    else {
        //bad hash collision
        //this candidate is not magic!
        return None
    }

    //Carry-Rippler
    blocker_subset = blocker_subset.wrapping_sub(all_blockers_set) & all_blockers_set;
    if blocker_subset == 0 {
        break;
    }
    }
    //no bad hash collisions
    //this candidate is magic!
    dbg!(lookup.len());
    Some(lookup)
}

//lazy_static! {
//    //vector containing a magic number and lookup table for each square
//    static ref ROOK_MAGIC_MASK: Vec<(u64, Vec<u64>)> = {
//        let mut mask: Vec<(u64, Vec<u64>)> = vec![(0, vec![]); 64];
//        for square in 0..64 as u8{
//            println!("finding rook magic for square {}...",square);
//            mask[square as usize] = find_magic(Rook, square);
//        }
//        mask
//    };
//}  
//
//lazy_static! {
//    //vector containing a magic number and lookup table for each square
//    static ref BISHOP_MAGIC_MASK: Vec<(u64, Vec<u64>)> = {
//        let mut mask: Vec<(u64, Vec<u64>)> = vec![(0, vec![]); 64];
//        for square in 0..64 as u8{
//            println!("finding bishop magic for square {}...",square);
//            mask[square as usize] = find_magic(Bishop, square);
//        }
//        mask
//    };
//}

//} 

//lazy_static! {
//    static ref ROOK_MAGIC_MASK: [(u64, [u64; 4096]); 64] = {
//        let mut mask: [(u64, [u64; 4096]); 64] = [(0, [0; 512]); 64];
//        for square in 0..64 as u8{
//            println!("finding rock magic for square {}...",square);
//            let (magic_number, lookup) = find_magic(Rook, square);
//            mask[square as usize].0 = magic_number;
//            for i in 0..lookup.len() {
//                mask[square as usize].1[i] = lookup[i];
//            }
//        }
//        mask
//    };
//} 

//lazy_static could not handle an array of this size, so it is time for a static mut
//would be nice to find an alternative to unsafe

static mut ROOK_MAGIC_MASK: [(u64, [u64; 4096]); 64] = [(0,[0; 4096]); 64];

//must be run to initialize the rook magic mask
//for safety, run this function before doing anything else
pub unsafe fn init_rook_magic_mask() {
    let mut mask: [(u64, [u64; 4096]); 64] = [(0, [0; 4096]); 64];
        for square in 0..64 as u8{
            println!("finding rook magic for square {}...",square);
            let (magic_number, lookup) = find_magic(Rook, square);
            mask[square as usize].0 = magic_number;
            for i in 0..lookup.len() {
                mask[square as usize].1[i] = lookup[i];
            }
        }
        ROOK_MAGIC_MASK = mask; 
}


lazy_static! {
    static ref BISHOP_MAGIC_MASK: [(u64, [u64; 512]); 64] = {
        let mut mask: [(u64, [u64; 512]); 64] = [(0, [0; 512]); 64];
        for square in 0..64 as u8{
            println!("finding bishop magic for square {}...",square);
            let (magic_number, lookup) = find_magic(Bishop, square);
            mask[square as usize].0 = magic_number;
            for i in 0..lookup.len() {
                mask[square as usize].1[i] = lookup[i];
            }
        }
        mask
    };
} 

//returns the amount of nodes given a position and a depth
pub fn perft(pos: &Position, depth: u8) -> usize {
    if depth == 1 {
        return pos.legal_moves.len();
    }
    let mut count = 0;

    for m in pos.clone().legal_moves.into_iter() {
        let mut pos_clone = pos.clone();
        if pos.w_turn {
            pos_clone.make_w_move(m);
        }
        else {
            pos_clone.make_b_move(m); 
        }
        count += perft(&pos_clone, depth-1);   
    }
    count
}


// --------------------------
// ######### TESTS ##########
// --------------------------
//------------------------------------------------------------------
//IMPORTANT to run these tests use RUST_MIN_STACK=8388608 cargo test 
//------------------------------------------------------------------
//running with the standard stack size for tests will cause overflow
//tests are slow due to initialization of magic numbers
//perft consists of positions made to catch movgen bugs from https://www.chessprogramming.org/Perft_Results
//where the nodecount at a certain depth is compared to the expected values

#[cfg(test)]
mod tests {
    use super::Position;
    use super::perft;
    use super::init_rook_magic_mask;
    use super::GameResult::*;
    
    #[test]
    fn perft1() {
        unsafe{init_rook_magic_mask()};
        assert_eq!(perft(&Position::startpos(), 1), 20);
        assert_eq!(perft(&Position::startpos(), 2), 400);
        assert_eq!(perft(&Position::startpos(), 3), 8902);
        assert_eq!(perft(&Position::startpos(), 4), 197281);
    }
    #[test]
    fn perft2() {
        unsafe{init_rook_magic_mask()};
        assert_eq!(perft(&Position::from_fen("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - "), 1), 14);
        assert_eq!(perft(&Position::from_fen("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - "), 2), 191);
        assert_eq!(perft(&Position::from_fen("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - "), 3), 2812);
        assert_eq!(perft(&Position::from_fen("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - "), 4), 43238);
    }

    #[test]
    fn perft3() {
        unsafe{init_rook_magic_mask()};
        assert_eq!(perft(&Position::from_fen("r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10 "), 1), 46);
        assert_eq!(perft(&Position::from_fen("r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10 "), 2), 2079);
        assert_eq!(perft(&Position::from_fen("r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10 "), 3), 89890);
        assert_eq!(perft(&Position::from_fen("r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10 "), 4), 3894594);
    }

    // example test
    // check that game state is in progress after initialisation
    #[test]
    fn game_in_progress_after_init() {
        assert_eq!(Position::startpos().game_in_progress(), true);
    }

    #[test]
    fn scolars_mate_from_startpos() {
        unsafe{init_rook_magic_mask()};
        let mut pos = Position::startpos();
        assert_eq!(pos.get_legal_moves().len(), 20);
        assert_eq!(pos.game_in_progress(), true);
        pos.make_move("e2e4");
        assert_eq!(pos.game_in_progress(), true);
        pos.make_move("e7e5");
        assert_eq!(pos.game_in_progress(), true);
        pos.make_move("d1h4");
        assert_eq!(pos.game_in_progress(), true);
        pos.make_move("b8c6");
        assert_eq!(pos.game_in_progress(), true);
        pos.make_move("f1c4");
        assert_eq!(pos.game_in_progress(), true);
        pos.make_move("g8f6");
        assert_eq!(pos.game_in_progress(), true);
        pos.make_move("h4f7");
        assert_eq!(pos.game_in_progress(), false);
        assert_eq!(pos.get_result(), WhiteWin)
    }

}
