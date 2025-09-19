use std::{collections::HashSet, sync::{Arc, Mutex}};

use crate::core::{board::Board, queue::{DistributedQueue, Queue}, set::Set};

pub type PositionsToEvaluate = DistributedQueue<(Option<Board>, Board, usize)>;
pub type PositionsToReevaluate = Set<Board>;