use crate::cpu::CPUConfig;
use crate::instructions::instructions::{WordType};

struct StoreBufferEntry {
    value: WordType,
    addr: WordType,
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
    pub(crate) fn new(cpu_config: &CPUConfig) -> StoreBuffer {
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

    pub(crate) fn size(&self) -> u16 {
        return (self.tail - self.head) as u16;
    }

    pub(crate) fn has_space(&self) -> bool {
        return self.size() < self.capacity;
    }

    pub(crate) fn allocate(&mut self) -> u16 {
        assert!(self.has_space(), "StoreBuffer: can't allocate because there is no space");

        let index = (self.tail % self.capacity as u64) as usize;
        self.tail += 1;
        return index as u16;
    }

    pub(crate) fn store(&mut self, index: u16, addr: WordType, value: WordType) {
        let sb_entry = &mut self.entries[index as usize];
        sb_entry.addr = addr;
        sb_entry.value = value;
        sb_entry.completed = true;
    }

    pub(crate) fn do_cycle(&mut self, memory: &mut Vec<WordType>) {
        for _ in 0..self.lfb_count {
            if self.tail == self.head {
                // store buffer is empty
                break;
            }

            let index = (self.head % self.capacity as u64) as usize;
            let mut sb_entry = &mut self.entries[index];
            if !sb_entry.completed {
                // the store buffer isn't empty, but there is a slot that didn't receive a store yet
                // We stop, so that we ensure that all stores in the store buffer, will be written
                // to memory in program order.
                return;
            }

            memory[sb_entry.addr as usize] = sb_entry.value;

            sb_entry.completed = false;
            sb_entry.value = 0;
            sb_entry.addr = 0;

            self.head += 1;
        }
    }
}
