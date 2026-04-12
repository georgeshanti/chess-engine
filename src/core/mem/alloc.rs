use std::{alloc::GlobalAlloc, thread::sleep, time::{Duration, Instant}};

const USIZE_STRING_SIZE: usize  = size_of::<usize>()*2;
const ADDRESS_VALUE_OFFSET: usize = 9;
const ALIGN_LABEL_OFFSET: usize= 9 + USIZE_STRING_SIZE;
const ALIGN_VALUE_OFFSET: usize= 9 + USIZE_STRING_SIZE + 10;
const SIZE_LABEL_OFFSET: usize= 9 + USIZE_STRING_SIZE + 10 + USIZE_STRING_SIZE;
const SIZE_VALUE_OFFSET: usize= 9 + USIZE_STRING_SIZE + 10 + USIZE_STRING_SIZE + 9;
const LINE_SIZE: usize = 9 + USIZE_STRING_SIZE + 10 + USIZE_STRING_SIZE + 9 + USIZE_STRING_SIZE + 1;

pub struct CustomAlloc<T: GlobalAlloc> {
    pub allocator: T
}

unsafe impl<T: GlobalAlloc> GlobalAlloc for CustomAlloc<T> {
    unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
        let ptr = unsafe { self.allocator.alloc(layout) };

        let mut file = std::fs::OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open("./logs/mem/log.txt")
        .unwrap();
        let mut line: [u8; LINE_SIZE] = [0; LINE_SIZE];
        line[0..9].copy_from_slice("Alloc: 0x".as_bytes());
        line[ADDRESS_VALUE_OFFSET..ALIGN_LABEL_OFFSET].copy_from_slice(&convert_to_hex(ptr as usize));

        line[ALIGN_LABEL_OFFSET..ALIGN_VALUE_OFFSET].copy_from_slice(" align: 0x".as_bytes());
        line[ALIGN_VALUE_OFFSET..SIZE_LABEL_OFFSET].copy_from_slice(&convert_to_hex(layout.align()));

        line[SIZE_LABEL_OFFSET..SIZE_VALUE_OFFSET].copy_from_slice(" size: 0x".as_bytes());
        line[SIZE_VALUE_OFFSET..LINE_SIZE-1].copy_from_slice(&convert_to_hex(layout.size()));
        line[LINE_SIZE-1] = b'\n';

        std::io::Write::write(&mut file, &line).unwrap();
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: std::alloc::Layout) {

        let mut file = std::fs::OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open("./logs/mem/log.txt")
        .unwrap();
        let mut line: [u8; LINE_SIZE] = [0; LINE_SIZE];
        line[0..9].copy_from_slice("Dlloc: 0x".as_bytes());
        line[ADDRESS_VALUE_OFFSET..ALIGN_LABEL_OFFSET].copy_from_slice(&convert_to_hex(ptr as usize));

        line[ALIGN_LABEL_OFFSET..ALIGN_VALUE_OFFSET].copy_from_slice(" align: 0x".as_bytes());
        line[ALIGN_VALUE_OFFSET..SIZE_LABEL_OFFSET].copy_from_slice(&convert_to_hex(layout.align()));

        line[SIZE_LABEL_OFFSET..SIZE_VALUE_OFFSET].copy_from_slice(" size: 0x".as_bytes());
        line[SIZE_VALUE_OFFSET..LINE_SIZE-1].copy_from_slice(&convert_to_hex(layout.size()));
        line[LINE_SIZE-1] = b'\n';

        std::io::Write::write(&mut file, &line).unwrap();
        unsafe { self.allocator.dealloc(ptr, layout) }
    }
}

const chars: [u8; 16] = [b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'a', b'b', b'c', b'd', b'e', b'f'];

pub fn convert_to_hex(val: usize) -> [u8; size_of::<usize>()*2] {
    let mut array = [0; size_of::<usize>()*2];
    let mut i = 0;
    for byte in val.to_be_bytes() {
        let high_byte = (byte & 0b11110000) >> 4;
        let low_byte = byte & 0b00001111;
        // println!("{:#x}", high_byte);
        // println!("{:#x}", low_byte);
        array[i*2] = chars[high_byte as usize];
        array[(i*2)+1] = chars[low_byte as usize];
        i = i + 1;
    }
    array
}

pub fn wait(d: Duration) {
    let t = Instant::now();
    while t.elapsed() < d {}
}