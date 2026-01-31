pub fn and_byte(value: [u8; 64], operand: u8) -> [u8; 64] {
    let operand: [u8; 8] = [operand; 8];
    let mut result: [u8; 64] = [0; 64];

    for i in 0..8 {
        let sub_value = &raw const value[i*8] as *const u64;
        let sub_operand = &raw const operand as *const u64;
        let sub_destination = &raw mut result[i*8] as *mut u64;
        unsafe {*sub_destination = *sub_value & *sub_operand;}
    }

    result
}

pub fn xor_byte(value: [u8; 64], operand: u8) -> [u8; 64] {
    let operand: [u8; 8] = [operand; 8];
    let mut result: [u8; 64] = [0; 64];

    for i in 0..8 {
        let sub_value = &raw const value[i*8] as *const u64;
        let sub_operand = &raw const operand as *const u64;
        let sub_destination = &raw mut result[i*8] as *mut u64;
        unsafe {*sub_destination = *sub_value ^ *sub_operand;}
    }

    result
}