use crate::cpu::CPUConfig;
use crate::instructions::{MemoryType, WordType};


struct StoreBufferEntry{
    value: Option<WordType>,
    addr: MemoryType,
}

pub(crate) struct StoreBuffer {

}

impl StoreBuffer{
    fn do_cycle(&self){

    }
}

pub(crate) struct MemorySubsystem{
    memory: Vec<WordType>,
    sb: StoreBuffer,
}

impl MemorySubsystem {

    pub fn new(cpu_config: &CPUConfig) -> MemorySubsystem {
        let mut memory = Vec::with_capacity(cpu_config.memory_size as usize);

        for _ in 0..cpu_config.memory_size{
            memory.push(0);
        }

        let sb = StoreBuffer{};

        MemorySubsystem {
            memory,
            sb,
        }
    }

    pub fn do_cycle(&mut self){
        self.sb.do_cycle();
    }
}



