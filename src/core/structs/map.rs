use std::{cmp, collections::{HashMap, HashSet}, hash::{DefaultHasher, Hash, Hasher, RandomState}, sync::{Arc, RwLock, Weak}};

use crate::{core::{chess::{board::{Board, BoardArrangement}, board_state::BoardState}, engine::structs::TimestampedEvaluation, structs::cash::Cash}, log};

pub struct PointerToBoard {
    pub ptr: Weak<RwLock<BoardArrangementPositions>>,
    pub index: usize,
}

#[derive(Clone)]
pub struct GroupedPositions {
    pub length: usize,
    pub map: [Option<Positions>; 16],
}

impl GroupedPositions {

    pub fn new(len: usize) -> Self {
        let mut gp = GroupedPositions {
            length: len,
            map: [const { None }; 16],
        };
        for i in 0..len {
            gp.map[i] = Some(Positions::new())
        }
        gp
    }

    pub fn edit(&self, index: usize, board: &Board) -> Presence<PointerToBoard> {
        self.map[index].clone().unwrap().edit(board)
    }

    pub fn is_board_arrangement_present(&self, board: &Board) -> bool {
        let hash = board.cash();
        let index: usize = (hash % (self.length as u64)) as usize;
        self.map[index].clone().unwrap().map.read().unwrap().contains_key(&board.get_board_arrangement())
    }

    pub fn get(&self, board: &Board) -> Option<PointerToBoard> {
        let hash = board.cash();
        let index: usize = (hash % (self.length as u64)) as usize;
        self.map[index].clone().unwrap().get(board)
    }

    pub fn len(self: &Self) -> [(usize, usize); 16] {
        let mut lens = [(0, 0); 16];
        for i in 0..self.length {
            let position = self.map[i].clone().unwrap();
            lens[i] = position.len();
        }
        lens
    }
}

#[derive(Clone)]
pub struct Positions {
    pub map: Arc<RwLock<HashMap<
        BoardArrangement,
        Arc<RwLock<BoardArrangementPositions>>
    >>>
}

pub const PAGE_SIZE: usize = 4096 * 100;
pub const PAGE_BOARD_STATE_COUNT: usize = PAGE_SIZE / size_of::<RwLock<BoardState>>();

// pub const PAGE_BOARD_COUNT: usize = PAGE_SIZE / size_of::<RwLock<BoardState>>();
pub const PAGE_BOARD_COUNT: usize = 4096;

pub struct BoardArrangementPositions {
    pub position_map: HashMap<Board, usize>,
    pub positions: [Option<Box<Vec<RwLock<BoardState>>>>; 128],
    pub positions_size: usize,

    pub next_moves: [Option<Vec<(Board, RwLock<Option<TimestampedEvaluation>>)>>; 512],
    pub next_moves_size: usize,
}

impl BoardArrangementPositions {
    pub fn new() -> Self {
        BoardArrangementPositions {
            position_map: HashMap::new(),
            positions: std::array::from_fn(|_| { None }),
            positions_size: 0,

            next_moves: std::array::from_fn(|_| { None }),
            next_moves_size: 0,
        }
    }

    pub fn get(&self, index: usize) -> &RwLock<BoardState> {
        let page_number = index / PAGE_BOARD_STATE_COUNT;
        let page_index = index % PAGE_BOARD_STATE_COUNT;
        let page = self.positions.get(page_number).unwrap().as_ref().unwrap();
        match page.get(page_index) {
            Some(board_state) => board_state,
            None => {
                panic!("Page index out of bounds: {} for page number: {} with capacity: {}", page_index, page_number, page.capacity());
            }
        }
    }

    pub fn get_next_moves(self: &Self, index: usize, size: usize, should_log: bool) -> Vec<&[(Board, RwLock<Option<TimestampedEvaluation>>)]> {
        let mut vec = Vec::with_capacity(size);
        let mut next_moves_index = index;
        let mut moves_left_to_fetch = size;
        if should_log {
            log!("moves_left_to_fetch: {}", moves_left_to_fetch);
        }
        while moves_left_to_fetch > 0 {
            if should_log {
                log!("moves_left_to_fetch: {}", moves_left_to_fetch);
            }
            let index = next_moves_index;
            let page = index / PAGE_BOARD_COUNT;
            let page_index = index % PAGE_BOARD_COUNT;

            let space_left = PAGE_BOARD_COUNT - page_index;
            let moves_to_read = cmp::min(space_left, moves_left_to_fetch);

            let page = self.next_moves.get(page).unwrap().as_ref().unwrap();
            let subslice = &page[page_index..(page_index+moves_to_read)];
            vec.push(subslice);

            next_moves_index += moves_to_read;
            moves_left_to_fetch -= moves_to_read;
        }
        vec
    }

    pub fn set_next_moves(self: &mut Self, next_moves: &[(Board, Option<TimestampedEvaluation>)]) -> usize {
        let start_index = self.next_moves_size;
        let mut next_moves_index = 0;
        let mut moves_left_to_insert = next_moves.len();
        while moves_left_to_insert > 0 {
            let index = self.next_moves_size;
            let page = index / PAGE_BOARD_COUNT;
            let pagen = page;
            let page_index = index % PAGE_BOARD_COUNT;
            if page_index == 0 {
                let k = self.next_moves.get_mut(page);
                let k = k.unwrap();
                *k = Some(Vec::with_capacity(PAGE_BOARD_COUNT));
            }
            let space_left = PAGE_BOARD_COUNT - page_index;
            let moves_to_write = cmp::min(space_left, moves_left_to_insert);
            let page = self.next_moves.get_mut(page).unwrap();
            let page = page.as_mut().unwrap();
            // let mut subslice = Vec::with_capacity(moves_to_write);
            for i in 0..moves_to_write {
                let next_move = next_moves[next_moves_index+i];
                page.push((next_move.0, RwLock::new(next_move.1)));
            }
            // let subslice: Box<[(Board, RwLock<Option<(crate::core::chess::board_state::Evaluation, std::time::Instant)>>)]> = next_moves[next_moves_index..(next_moves_index+moves_to_write)].iter().map(|t| (t.0, RwLock::new(t.1))).collect();
            // page.append(&mut subslice);

            next_moves_index += moves_to_write;
            moves_left_to_insert -= moves_to_write;
            self.next_moves_size += moves_to_write;
        }
        start_index
    }
}

pub enum Presence<T> {
    Present{value: T},
    Absent{value: T},
}

impl Positions {

    pub fn new() -> Self {
        Positions {
            map: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn get_board_arrangement_positions(&self, board: &Board) -> Arc<RwLock<BoardArrangementPositions>> {
        let readable_board_pieces_map = self.map.read().unwrap();
        let board_arrangement = board.get_board_arrangement();
        let board_pieces_map = readable_board_pieces_map.get(&board_arrangement);
        match board_pieces_map {
            Some(board_pieces_map) => board_pieces_map.clone(),
            None => {
                drop(readable_board_pieces_map);
                let mut writable_board_pieces_map = self.map.write().unwrap();
                match writable_board_pieces_map.get(&board_arrangement) {
                    Some(board_pieces_map) => board_pieces_map.clone(),
                    None => {
                        let new_board_pieces_map = Arc::new(RwLock::new(BoardArrangementPositions::new()));
                        writable_board_pieces_map.insert(board_arrangement, new_board_pieces_map.clone());
                        new_board_pieces_map
                    }
                }
            }
        }
    }

    pub fn get_board_arrangement_positions_or_none(&self, board: &Board) -> Option<Arc<RwLock<BoardArrangementPositions>>> {
        let readable_board_pieces_map = self.map.read().unwrap();
        let board_arrangement = board.get_board_arrangement();
        let board_pieces_map = readable_board_pieces_map.get(&board_arrangement);
        board_pieces_map.map(|board_pieces_map| board_pieces_map.clone())
    }

    // pub fn is_present(&self, board: &Board) -> bool {
    //     match self.map.read().unwrap().get(&get_board_pieces(board)) {
    //         Some(positions_map) => positions_map.read().unwrap().contains_key(board),
    //         None => false,
    //     }
    // }

    pub fn get(&self, board: &Board) -> Option<PointerToBoard> {
        let board_arrangement_positions = self.get_board_arrangement_positions_or_none(&board);
        board_arrangement_positions.and_then(|board_arrangement_positions| {
            let readable_board_arrangement_positions = board_arrangement_positions.read().unwrap();
            let index = readable_board_arrangement_positions.position_map.get(&board);
            match index {
                Some(index) => {
                    Some(PointerToBoard { ptr: Arc::downgrade(&board_arrangement_positions), index: *index })
                }
                None => None,
            }
        })
    }

    pub fn edit(& self, board: & Board) -> Presence<PointerToBoard> {
        // log!("Editing board");
        let board_arrangement_positions = self.get_board_arrangement_positions(board);
        // log!("Got board pieces map");
        let readable_board_arrangement_positions = board_arrangement_positions.read().unwrap();
        // log!("Got readable positions map");
        let board_state_position = readable_board_arrangement_positions.position_map.get(&board);
        if board_state_position.is_some() {
            let index = *board_state_position.unwrap();
            Presence::Present { value: PointerToBoard { ptr: Arc::downgrade(&board_arrangement_positions), index: index } }
        } else {
            drop(readable_board_arrangement_positions);
            let mut writable_board_arrangement_positions = board_arrangement_positions.write().unwrap();
            let board_state_position = writable_board_arrangement_positions.position_map.get(&board);
            if board_state_position.is_some() {
                let index = *board_state_position.unwrap();
                Presence::Present { value: PointerToBoard { ptr: Arc::downgrade(&board_arrangement_positions), index: index } }
            } else {
                let index = writable_board_arrangement_positions.positions_size;
                let page = index / PAGE_BOARD_STATE_COUNT;
                let page_index = index % PAGE_BOARD_STATE_COUNT;
                if page_index == 0 {
                    let k = writable_board_arrangement_positions.positions.get_mut(page);
                    if let None = k {
                        log!("Page not found: {}", page);
                    }
                    let k = k.unwrap();
                    *k = Some(Box::new(Vec::with_capacity(PAGE_BOARD_STATE_COUNT)));
                }
                let vec = writable_board_arrangement_positions.positions.get_mut(page).unwrap().as_mut().unwrap();
                vec.push(RwLock::new(BoardState::new()));
                writable_board_arrangement_positions.position_map.insert(*board, index);
                writable_board_arrangement_positions.positions_size += 1;
                Presence::Absent { value: PointerToBoard { ptr: Arc::downgrade(&board_arrangement_positions), index: index } }
            }
        }
    }

    pub fn len(&self) -> (usize, usize) {
        let mut len = 0;
        for (_, value) in self.map.read().unwrap().iter() {
            let board_arrangement_positions = value.read().unwrap();
            len = len + board_arrangement_positions.positions_size;
        }
        (self.map.read().unwrap().len(), len)
    }
}