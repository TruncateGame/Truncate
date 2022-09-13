use super::bag::TileBag;
use super::board::{Board, Coordinate, Square};
use super::hand::Hands;

pub enum Move {
    Place(usize, char, Coordinate),
    Swap(usize, Coordinate, Coordinate),
}

// TODO: is it weird to implement this on Board here rather than on Move?
impl Board {
    fn make_move<'a>(&'a mut self, game_move: Move, hands: &'a mut Hands) -> Result<(), &str> {
        match game_move {
            Move::Place(player, tile, position) => {
                match self.get(position) {
                    Err(_) => return Err("Couldn't get square"), // TODO: propogate the internal error, ideally succinctly with the ? operator. This is hard because of a borrow checker issue https://github.com/rust-lang/rfcs/blob/master/text/2094-nll.md#problem-case-3-conditional-control-flow-across-functions
                    Ok(sq) => match sq {
                        Square::Occupied(_, _) => {
                            return Err("Cannot place a tile in an occupied square")
                        }
                        Square::Empty => {}
                    },
                };

                if position != self.get_root(player)
                    && self
                        .neighbouring_squares(position)
                        .iter()
                        .filter(|square| match (*square).1 {
                            Square::Empty => false,
                            Square::Occupied(p, _) => *p == player,
                        })
                        .count()
                        == 0
                {
                    return Err("Must place tile on square that neighbours one of your already placed tiles, or on your root");
                }

                hands.use_tile(player, tile)?; // Use tile checks that the player is valid and has that letter
                self.set(position, player, tile)?;
                Ok(())
            }
            Move::Swap(player, position_1, position_2) => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_placement_locations() {
        let mut b = Board::new(3, 1);
        let mut hands = Hands::new(2, 7, TileBag::trivial_bag());

        let out_of_bounds = Move::Place(0, 'A', Coordinate { x: 10, y: 10 });
        assert_eq!(
            b.make_move(out_of_bounds, &mut hands),
            // Err("y-coordinate is too large for board height") // <- TODO
            Err("Couldn't get square")
        );

        let out_of_bounds = Move::Place(0, 'A', Coordinate { x: 10, y: 0 });
        assert_eq!(
            b.make_move(out_of_bounds, &mut hands),
            // Err("x-coordinate is too large for board width") // <- TODO
            Err("Couldn't get square")
        );

        let dead = Move::Place(0, 'A', Coordinate { x: 0, y: 0 });
        assert_eq!(b.make_move(dead, &mut hands), Err("Couldn't get square"));
    }

    #[test]
    fn can_place() {
        let mut b = Board::new(3, 1);
        let mut hands = Hands::new(2, 7, TileBag::trivial_bag());

        assert_eq!(
            b.make_move(Move::Place(0, 'A', Coordinate { x: 1, y: 0 }), &mut hands),
            Ok(())
        );
        assert_eq!(
            b.make_move(Move::Place(0, 'A', Coordinate { x: 1, y: 0 }), &mut hands),
            Err("Cannot place a tile in an occupied square")
        );
    }

    #[test]
    fn invalid_player_or_tile() {
        let mut b = Board::new(3, 1);
        let mut hands = Hands::default();

        assert_eq!(
            b.make_move(Move::Place(2, 'A', Coordinate { x: 1, y: 0 }), &mut hands),
            Err("Invalid player")
        );

        assert_eq!(
            b.make_move(Move::Place(0, '&', Coordinate { x: 1, y: 0 }), &mut hands),
            Err("Player doesn't have that tile")
        );
    }
}
