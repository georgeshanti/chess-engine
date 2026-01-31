

fn main2() {
    let current_board_arrangement: BoardArrangement = serde_json::from_str("{\"higher\":{\"pawns\":65280,\"major_pieces\":[8,2,2,2,1,1]},\"lower\":{\"pawns\":65280,\"major_pieces\":[8,2,2,2,1,1]}}").unwrap();
    let board_arrangement: BoardArrangement = serde_json::from_str("{\"higher\":{\"pawns\":8421120,\"major_pieces\":[8,2,2,2,1,1]},\"lower\":{\"pawns\":195840,\"major_pieces\":[8,2,2,2,1,1]}}").unwrap();
    println!("{}", current_board_arrangement);
    println!("{}", board_arrangement);
    println!("{}", can_come_after_board_arrangement(&current_board_arrangement, &board_arrangement));
}

fn main1() {
    {
        let f = format!("logs/{}.log", chrono::Local::now().format("%Y-%m-%d_%H-%M-%S").to_string());
        let mut file_name = FILENAME.write().unwrap();
        *file_name = f;
    }
    let queue = WeightedQueue::new();
    queue.queue(vec![INITIAL_BOARD], 0);
    let positions = Positions::new();
    loop {
        let d = {
            let mut c = 0;
            loop {
                if c > 10 {
                    break None;
                }
                match queue.dequeue_optional() {
                    Some(value) => {
                        break Some(value);
                    }
                    None => {
                        sleep(Duration::from_millis(100));
                        c += 1;
                    }
                }
            }
        };
        match d {
            Some(value) => {
                println!("Dequeued: {}", value.0);
                if value.0 <= 2 {
                    for board in value.1.iter() {
                        let evaluation = board.get_evaluation();
                        positions.edit(&board);
                        queue.queue(evaluation.1.to_vec(), value.0 + 1);
                    }
                }
            }
            None => {
                println!("Queue is empty");
                break;
            }
        }
        println!("Queue length: {}", queue.len());
    }
    println!("Positions: {}", positions.len());
    let current_board = INITIAL_BOARD.clone().get_evaluation().1[11];
    let keys = { let t = positions.map.read().unwrap(); t.keys().map(|f| f.clone()).collect::<Vec<BoardArrangement>>() };
    for key in keys {
        if !can_come_after_board_arrangement(&current_board.get_board_arrangement(), &key) {
            positions.map.write().unwrap().remove(&key);
        }
    }
    println!("Positions: {}", positions.len());
    for key in positions.map.read().unwrap().keys() {
        log!("\n{}", key);
    }

}