use crate::cpu::CPUConfig;
use crate::instructions::{MemoryType, WordType};


struct StoreBufferEntry {
    value: WordType,
    addr: MemoryType,
    completed: bool,
}

pub(crate) struct StoreBuffer {
    head: u64,
    tail: u64,
    entries: Vec<StoreBufferEntry>,
    capacity: u16,
    lfb_count: u8,
}

impl StoreBuffer {
    pub fn new(cpu_config: &CPUConfig) -> StoreBuffer {
        let mut entries = Vec::with_capacity(cpu_config.sb_capacity as usize);
        for _ in 0..cpu_config.sb_capacity {
            entries.push(StoreBufferEntry {
                value: 0,
                addr: 0,
                completed: false,
            })
        }

        StoreBuffer {
            capacity: cpu_config.sb_capacity,
            head: 0,
            tail: 0,
            entries,
            lfb_count: cpu_config.lfb_count,
        }
    }

    pub fn allocate(&mut self) -> u16 {
        let index = (self.tail % self.capacity as u64) as usize;
        let sb_entry = &mut self.entries[index];
        sb_entry.completed = false;
        let res = self.tail;
        self.tail += 1;
        return index as u16;
    }

    pub fn store(&mut self, index: usize, addr: MemoryType, value: WordType) {
        let sb_entry = &mut self.entries[index];
        sb_entry.addr = addr;
        sb_entry.value = value;
        sb_entry.completed = true;
    }

    fn do_cycle(&mut self, memory: &mut Vec<WordType>) {
        for _ in 0..self.lfb_count {
            if self.tail == self.head {
                // store buffer is empty
                return;
            }

            let index = (self.head % self.capacity as u64) as usize;
            let sb_entry = &self.entries[index];
            if !sb_entry.completed {
                // the store buffer isn't empty, but there is a slot that didn't receive a store yet
                // We stop, so that we ensure that all stores in the store buffer, will be written
                // to memory in program order.
                return;
            }

            memory[sb_entry.addr as usize] = sb_entry.value;
            self.head += 1;
        }
    }
}

pub(crate) struct MemorySubsystem {
    memory: Vec<WordType>,
    pub(crate) sb: StoreBuffer,
}

impl MemorySubsystem {
    pub fn new(cpu_config: &CPUConfig) -> MemorySubsystem {
        let mut memory = Vec::with_capacity(cpu_config.memory_size as usize);

        for _ in 0..cpu_config.memory_size {
            memory.push(0);
        }

        let sb = StoreBuffer::new(cpu_config);

        MemorySubsystem {
            memory,
            sb,
        }
    }

    pub fn do_cycle(&mut self) {
        self.sb.do_cycle(&mut self.memory);
    }
}



