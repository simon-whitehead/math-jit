extern crate memmap;

use std::{mem, ptr};
use std::ops::{Deref, DerefMut};

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

    fn write_code_at(&mut self, code: &[u8], position: usize) {
        for (src, dst) in code.iter().zip(self.code[position..].iter_mut()) {
            *dst = *src;
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

    pub fn write_u64_at(&mut self, num: u64, position: usize) {
        let b1 = (num & 0xFF) as u8;
        let b2 = ((num & 0xFF00) >> 0x08) as u8;
        let b3 = ((num & 0xFF0000) >> 0x10) as u8;
        let b4 = ((num & 0xFF000000) >> 0x18) as u8;
        let b5 = ((num & 0xFF00000000) >> 0x20) as u8;
        let b6 = ((num & 0xFF0000000000) >> 0x28) as u8;
        let b7 = ((num & 0xFF000000000000) >> 0x30) as u8;
        let b8 = ((num & 0xFF00000000000000) >> 0x38) as u8;

        self.write_code_at(&[b1, b2, b3, b4, b5, b6, b7, b8], position);
    }

    pub fn execute<T>(&mut self) -> T {
        let mut memory_map = Mmap::anonymous(self.code.len(), Protection::ReadWrite).unwrap();

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
    add_code: ExecutableMemory,
}

impl JITCalculator {
    pub fn new() -> JITCalculator {
        JITCalculator { add_code: ExecutableMemory::new() }
    }

    /// Clears the memory map
    fn clear_mem(&mut self) {
        self.add_code.clear_mem();
    }

    // Generates a "Add two numbers together" function in memory
    // and uses the arguments passed in as the memory locations to
    // add them together
    pub fn add<T>(&mut self, left: &T, right: &T) -> T {
        let left_addr = left as *const T as usize;
        let right_addr = right as *const T as usize;

        if self.add_code.is_empty() {
            self.add_code.write_code(&[0x48, 0xb8]);        /* mov rax,imm64 */
            self.add_code.write_u64(left_addr as u64);     /* imm64 == address of left variable */

            self.add_code.write_code(&[0x48, 0xbb]);       /* mov rbx,imm64 */
            self.add_code.write_u64(right_addr as u64);    /* imm64 == address of right variable */

            self.add_code.write_code(&[0x48, 0x8b, 0x08]); /* mov rcx,[rax] */
            self.add_code.write_code(&[0x48, 0x8b, 0x13]); /* mov rdx,[rbx] */

            self.add_code.write_code(&[0x48, 0x01, 0xd1]); /* add rcx,rdx */
            self.add_code.write_code(&[0x48, 0x89, 0xc8]); /* mov rax,rcx */
            self.add_code.write_code(&[0xc3]);             /* ret */

            self.add_code.execute::<T>()
        } else {
            self.add_code.write_u64_at(left_addr as u64, 0x02);
            self.add_code.write_u64_at(right_addr as u64, 0x0C);

            self.add_code.execute::<T>()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::mem;

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

        let result = executable_memory.execute::<u32>();

        assert_eq!(16, result);
    }

    #[test]
    fn jit_calculator() {
        let mut calc = JITCalculator::new();

        let a = &mut 10;
        let b = &mut 22;

        let result = calc.add::<u64>(a, b);

        assert_eq!(32, result);
    }
}