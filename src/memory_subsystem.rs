use crate::cpu::CPUConfig;
use crate::instructions::WordType;

pub(crate) struct MemorySubsystem{
    memory: Vec<WordType>,
}

impl MemorySubsystem {
    pub fn new(cpu_config: &CPUConfig) -> MemorySubsystem {
        let mut memory = Vec::with_capacity(cpu_config.memory_size as usize);

        for _ in 0..cpu_config.memory_size{
            memory.push(0);
        }

        MemorySubsystem {
            memory
        }
    }
}



