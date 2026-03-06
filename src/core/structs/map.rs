use std::{collections::{HashMap, HashSet}, fs::{File, remove_file}, hash::{DefaultHasher, Hash, Hasher, RandomState}, sync::{Arc, RwLock, Weak}};

use crate::{core::{chess::{board::{Board, BoardArrangement}, board_state::BoardState}, structs::{cash::Cash, lru::{ArrayInternal, Loader, Lru}}}, log};
use chrono::Utc;
use serde::{Serialize, Deserialize};
use serde_big_array::{Array, BigArray};
use serde_with::serde_as;

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

    pub fn len(self: &Self) -> String {
        let mut lens = vec![];
        for i in 0..self.length {
            let position = self.map[i].clone().unwrap();
            lens.push(position.len());
        }
        let t = lens.join(", ");
        format!("{{{}}}", t)
    }
}

pub static COLD_CACHE: RwLock<String> = RwLock::new(String::new());

pub struct Mapper {}

impl Loader<Arc<RwLock<BoardArrangementPositions>>, String> for Mapper{
    fn load(cold: &String) -> Arc<RwLock<BoardArrangementPositions>> {
        let file_path = format!("./{}/{}", COLD_CACHE.read().unwrap(), cold);
        let reader = File::open(file_path.clone()).unwrap();
        log!("file_path: {}", file_path);
        let hot = serde_json::from_reader(reader).unwrap();
        remove_file(cold).unwrap();
        return Arc::new(RwLock::new(hot));
    }

    fn store(hot: &Arc<RwLock<BoardArrangementPositions>>) -> String {
        let file_name = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S").to_string();
        let file_path = format!("./{}/{}", COLD_CACHE.read().unwrap(), file_name);
        log!("{}", file_path);
        let writer = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open(file_path).unwrap();
        let board_arrangement_positions = hot.read().unwrap();
        let t = &*board_arrangement_positions;
        serde_json::to_writer(writer, t).unwrap();
        file_name
    }
}

#[derive(Clone)]
pub struct Positions {
    pub map: Arc<RwLock<HashMap<
        BoardArrangement,
        usize
    >>>,
    pub array: Arc<RwLock<ArrayInternal<Arc<RwLock<BoardArrangementPositions>>, String, Mapper>>>,
}

pub const PAGE_SIZE: usize = 4096 * 1024;
pub const PAGE_BOARD_COUNT: usize = PAGE_SIZE / size_of::<RwLock<BoardState>>();

#[serde_as]
#[derive(Serialize, Deserialize)]
pub struct BoardArrangementPositions {
    #[serde_as(as = "Vec<(_, _)>")]
    pub map: HashMap<Board, usize>,
    #[serde(with = "BigArray")]
    pub positions: [Option<Box<Vec<RwLock<BoardState>>>>; 128],
    pub size: usize,
}

impl BoardArrangementPositions {
    pub fn new() -> Self {
        BoardArrangementPositions {
            map: HashMap::new(),
            positions: std::array::from_fn(|_| { None }),
            size: 0,
        }
    }

    pub fn get(&self, index: usize) -> &RwLock<BoardState> {
        let page_number = index / PAGE_BOARD_COUNT;
        let page_index = index % PAGE_BOARD_COUNT;
        let page = self.positions.get(page_number).unwrap().as_ref().unwrap();
        match page.get(page_index) {
            Some(board_state) => board_state,
            None => {
                panic!("Page index out of bounds: {} for page number: {} with capacity: {}", page_index, page_number, page.capacity());
            }
        }
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
            array: Arc::new(RwLock::new(ArrayInternal::new(Mapper{}))),
        }
    }

    pub fn get_board_arrangement_positions(&self, board: &Board) -> Arc<RwLock<BoardArrangementPositions>> {
        let readable_board_pieces_map = self.map.read().unwrap();
        let board_arrangement = board.get_board_arrangement();
        let board_pieces_map = readable_board_pieces_map.get(&board_arrangement);
        let mut array = self.array.write().unwrap();
        match board_pieces_map {
            Some(board_pieces_map) => array.get(*board_pieces_map).clone(),
            None => {
                drop(readable_board_pieces_map);
                let mut writable_board_pieces_map = self.map.write().unwrap();
                match writable_board_pieces_map.get(&board_arrangement) {
                    Some(board_pieces_map) => array.get(*board_pieces_map).clone(),
                    None => {
                        let new_board_pieces_map = Arc::new(RwLock::new(BoardArrangementPositions::new()));
                        let index = array.push(new_board_pieces_map.clone());
                        writable_board_pieces_map.insert(board_arrangement, index);
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
        let mut array = self.array.write().unwrap();
        board_pieces_map.map(|board_pieces_map| {
            array.get(*board_pieces_map).clone()
        })
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
            let index = readable_board_arrangement_positions.map.get(&board);
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
        let board_state_position = readable_board_arrangement_positions.map.get(&board);
        if board_state_position.is_some() {
            let index = *board_state_position.unwrap();
            Presence::Present { value: PointerToBoard { ptr: Arc::downgrade(&board_arrangement_positions), index: index } }
        } else {
            drop(readable_board_arrangement_positions);
            let mut writable_board_arrangement_positions = board_arrangement_positions.write().unwrap();
            let board_state_position = writable_board_arrangement_positions.map.get(&board);
            if board_state_position.is_some() {
                let index = *board_state_position.unwrap();
                Presence::Present { value: PointerToBoard { ptr: Arc::downgrade(&board_arrangement_positions), index: index } }
            } else {
                let index = writable_board_arrangement_positions.size;
                let page = index / PAGE_BOARD_COUNT;
                let page_index = index % PAGE_BOARD_COUNT;
                if page_index == 0 {
                    let k = writable_board_arrangement_positions.positions.get_mut(page);
                    if let None = k {
                        log!("Page not found: {}", page);
                    }
                    let k = k.unwrap();
                    *k = Some(Box::new(Vec::with_capacity(PAGE_BOARD_COUNT)));
                    // log!("Created new page");
                }
                let vec = writable_board_arrangement_positions.positions.get_mut(page).unwrap().as_mut().unwrap();
                vec.push(RwLock::new(BoardState::new()));
                writable_board_arrangement_positions.map.insert(*board, index);
                writable_board_arrangement_positions.size += 1;
                Presence::Absent { value: PointerToBoard { ptr: Arc::downgrade(&board_arrangement_positions), index: index } }
            }
        }
    }

    pub fn len(&self) -> String {
        format!("{}", self.map.read().unwrap().len())
    }
}