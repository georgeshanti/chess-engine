use std::{collections::HashSet, sync::{Arc, Mutex}};

use crate::core::{board::Board, queue::Queue, set::Set};

pub type PositionsToEvaluate = Queue<(Option<Board>, Board, usize)>;
pub type PositionsToReevaluate = Set<Board>;