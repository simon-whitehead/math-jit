extern crate byteorder;
extern crate memmap;

use std::{mem, ptr};
use std::ops::{Deref, DerefMut};

use byteorder::{ByteOrder, LittleEndian};
use memmap::{Mmap, Protection};

pub struct ExecutableMemory {
    code: Vec<u8>,
    memory_map: Option<Mmap>,
}

impl ExecutableMemory {
    pub fn new() -> ExecutableMemory {
        ExecutableMemory {
            code: Vec::new(),
            memory_map: None,
        }
    }

    fn clear_mem(&mut self) {
        self.code.clear();
    }

    pub fn write_code(&mut self, code: &[u8]) {
        for byte in code {
            self.code.push(*byte);
        }
    }

    pub fn write_u64(&mut self, num: u64) {
        let b1 = (num & 0xFF) as u8;
        let b2 = ((num & 0xFF00) >> 0x08) as u8;
        let b3 = ((num & 0xFF0000) >> 0x10) as u8;
        let b4 = ((num & 0xFF000000) >> 0x18) as u8;
        let b5 = ((num & 0xFF00000000) >> 0x20) as u8;
        let b6 = ((num & 0xFF0000000000) >> 0x28) as u8;
        let b7 = ((num & 0xFF000000000000) >> 0x30) as u8;
        let b8 = ((num & 0xFF00000000000000) >> 0x38) as u8;

        self.write_code(&[b1, b2, b3, b4, b5, b6, b7, b8]);
    }

    pub fn compile(&mut self) {
        self.memory_map = Some(Mmap::anonymous(self.code.len(), Protection::ReadWrite).unwrap());
    }

    pub fn execute<T>(&mut self) -> T {
        let mut memory_map = self.memory_map.as_mut().unwrap();

        unsafe {
            ptr::copy(self.code.as_ptr(), memory_map.mut_ptr(), self.code.len());
            memory_map.set_protection(Protection::ReadExecute);
            mem::transmute::<_, fn() -> T>(memory_map.ptr())()
        }
    }
}

impl Deref for ExecutableMemory {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        &self.code
    }
}

impl DerefMut for ExecutableMemory {
    fn deref_mut(&mut self) -> &mut [u8] {
        &mut self.code
    }
}

pub struct JITCalculator {
    memory: ExecutableMemory,
}

impl JITCalculator {
    pub fn new() -> JITCalculator {
        JITCalculator { memory: ExecutableMemory::new() }
    }

    /// Clears the memory map
    fn clear_mem(&mut self) {
        self.memory.clear_mem();
    }

    // Generates a "Add two numbers together" function in memory
    // and uses the arguments passed in as the memory locations to
    // add them together
    pub fn add<T>(&mut self, left: &T, right: &T) -> T {
        let left_addr: usize = usize::from_str_radix(&format!("{:p}", left)[2..], 16).unwrap();
        let right_addr: usize = usize::from_str_radix(&format!("{:p}", right)[2..], 16).unwrap();

        self.memory.write_code(&[0x48, 0xb8);        /* mov rax,imm64 */]
        self.memory.write_u64(left_addr as u64);     /* imm64 == address of left variable */

        self.memory.write_code(&[0x48, 0xbb]);       /* mov rbx,imm64 */
        self.memory.write_u64(right_addr as u64);    /* imm64 == address of right variable */

        self.memory.write_code(&[0x48, 0x8b, 0x08]); /* mov rcx,[rax] */
        self.memory.write_code(&[0x48, 0x8b, 0x13]); /* mov rdx,[rbx] */

        self.memory.write_code(&[0x48, 0x01, 0xd1]); /* add rcx,rdx */
        self.memory.write_code(&[0x48, 0x89, 0xc8]); /* mov rax,rcx */
        self.memory.write_code(&[0xc3]);             /* ret */

        self.memory.compile();

        self.memory.execute::<T>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let mut executable_memory = ExecutableMemory::new();
        executable_memory.write_code(
            &[
                0xb8, 0x05, 0x00, 0x00, 0x00,   // mov eax, 5
                0xba, 0x0b, 0x00, 0x00, 0x00,   // mov edx, 11
                0x01, 0xd0,                     // add eax, edx
                0xc3                            // ret
            ]
        );

        executable_memory.compile();

        let result = executable_memory.execute::<u32>();

        assert_eq!(16, result);
    }

    #[test]
    fn jit_calculator() {
        let mut calc = JITCalculator::new();

        let a = &mut 10;
        let b = &mut 22;

        let result = calc.add(a, b);

        assert_eq!(32, result);

        *a = 40;
        *b = 33;

        let result = calc.add(a, b);

        assert_eq!(73, result);
    }
}
