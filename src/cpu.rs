use std::cell::RefCell;
use crate::instructions::{InstrQueue, Program};
use std::rc::Rc;
use std::thread;
use std::time::Duration;
use crate::backend::Backend;
use crate::frontend::Frontend;
use crate::memory_subsystem::MemorySubsystem;

pub(crate) struct CPUConfig {
    pub(crate) arch_reg_count: u16,
    pub(crate) phys_reg_count: u16,
    pub(crate) frontend_n_wide: u8,
    pub(crate) instr_queue_capacity: u16,
    pub(crate) frequency_hz: u64,
    pub(crate) rs_count: u16,
    pub(crate) memory_size: u32,
}

pub(crate) struct CPU<'a> {
    backend: Backend<'a>,
    frontend: Frontend<'a>,
    cycle_cnt: u64,
    cycle_period: Duration,
}

impl<'a> CPU<'a> {
    pub(crate) fn new(cpu_config: &'a CPUConfig) -> CPU<'a> {
        let instr_queue = Rc::new(RefCell::new(InstrQueue::new(cpu_config.instr_queue_capacity)));

        let memory_subsystem = Rc::new(RefCell::new(MemorySubsystem::new(cpu_config)));

        let backend = Backend::new(cpu_config,
                                   Rc::clone(&instr_queue),
                                   Rc::clone(&memory_subsystem));

        let frontend = Frontend::new(cpu_config,
                                     Rc::clone(&instr_queue));
        CPU {
            backend,
            frontend,
            cycle_cnt: 0,
            cycle_period: Duration::from_micros(1_000_000 / cpu_config.frequency_hz),
        }
    }

    pub(crate) fn run(&mut self, program: Program) {
        self.frontend.init(program);

        loop {
            self.cycle_cnt += 1;
            println!("At cycle {}", self.cycle_cnt);
            self.frontend.cycle();
            self.backend.cycle();
            thread::sleep(self.cycle_period);
        }
    }
}
