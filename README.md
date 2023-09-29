# linug-chess

important! rook magics need to be initialized with 

unsafe{init_rook_magic_mask()};

before using any other functions

create a new position with

Position::startpos() for standard chess position

Position::from_fen(fen_string) to parse position from a fen string

make a move with

Position.make_move(move)

example Position.make_move("e2e4")

to add make a promotion add q => Queen, r => Rook, b => Bishop, n => Knight

example Postion.make_move("a7a8q")


get all legal moves with 

Position.get_legal_moves()

get all legal moves from a square with

Position.get_square_legal_moves()

use Position.game_in_progress() to determine if game is in progress

if it is not use Position.get_resualt() to get the result
