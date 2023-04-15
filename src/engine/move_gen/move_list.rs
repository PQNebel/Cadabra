use super::*;

pub struct MoveList {
    moves: heapless::Vec<(Move, i16), 256>
}

impl MoveList {
    pub fn new() -> Self {
        Self {
            moves: heapless::Vec::new()
        }
    }

    pub fn push(&mut self, moov: Move) {
        unsafe { self.moves.push_unchecked((moov, 0)) }
    }

    pub fn pop(&mut self) -> Option<Move> {
        self.moves.pop().map(|e| e.0)
    }

    /// This will only make sense if the list has been sorted!
    pub fn pop_best(&mut self) -> Option<Move> {
        if self.moves.len() == 0 {
            return None
        }

        let mut best_index = 0;
        let mut best_score = self.moves[0].1;

        for i in 0..self.len() {
            let score = self.moves[i].1;

            if score > best_score {
                best_index = i;
                best_score = self.moves[best_index].1;
            }
        }

        Some(self.moves.swap_remove(best_index).0)
    }

    pub fn len(&self) -> usize {
        self.moves.len()
    }

    pub fn sort(&mut self) {
        
    }
}

impl Iterator for MoveList {
    type Item = Move;

    fn next(&mut self) -> Option<Self::Item> {
        self.pop()
    }
}