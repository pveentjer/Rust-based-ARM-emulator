use std::cell::RefCell;
use crate::instructions::{InstrQueue, Program, RegisterType, WordType};
use std::rc::Rc;
use std::thread;
use std::time::Duration;
use crate::backend::Backend;
use crate::frontend::{Frontend, FrontendControl};
use crate::memory_subsystem::MemorySubsystem;

#[derive(Clone)]
pub(crate) struct CPUConfig {
    // the number of architectural registers
    pub(crate) arch_reg_count: u16,
    // the number of physical registers
    pub(crate) phys_reg_count: u16,
    // the number of instructions the frontend can fetch/decode per clock cycle.
    pub(crate) frontend_n_wide: u8,
    // the size of the instruction queue between frontend and backend
    pub(crate) instr_queue_capacity: u16,
    // the frequence of the CPU in Hz.
    pub(crate) frequency_hz: u64,
    // the number of reservation stations
    pub(crate) rs_count: u16,
    // the size of the memory in machine words
    pub(crate) memory_size: u32,
    // the capacity of the store buffer
    pub(crate) sb_capacity: u16,
    // the number of line fill buffers; currently there are no line fill buffer
    // it is just a limit of the number of stores that can commit to memory
    // per clock cycle (there is also no cache)
    pub(crate) lfb_count: u8,
    // the capacity of the reorder buffer
    pub(crate) rob_capacity: u16,
    // the number of execution units
    pub(crate) eu_count: u8,
    // if processing of a single instruction should be traced (printed)
    pub(crate) trace: bool,
    // the number of instructions that can retire per clock cycle
    pub(crate) retire_n_wide: u8,
    // the number of instructions that can be dispatched (send to execution units) every clock cycle.
    pub(crate) dispatch_n_wide: u8,
    // the number of instructions that can be issued to  the rob or finding reservation stations, every clock cycle.
    pub(crate) issue_n_wide: u8,
}

pub(crate) struct CPU {
    backend: Backend,
    frontend: Frontend,
    memory_subsystem: Rc<RefCell<MemorySubsystem>>,
    arch_reg_file: Rc<RefCell<ArgRegFile>>,
    cycle_cnt: u64,
    cycle_period: Duration,
}

impl CPU {
    pub(crate) fn new(cpu_config: &CPUConfig) -> CPU {
        let instr_queue = Rc::new(RefCell::new(InstrQueue::new(cpu_config.instr_queue_capacity)));

        let memory_subsystem = Rc::new(RefCell::new(MemorySubsystem::new(cpu_config)));

        let arch_reg_file = Rc::new(RefCell::new(ArgRegFile::new(cpu_config.arch_reg_count)));

        let mut frontend_control = Rc::new(RefCell::new(FrontendControl{ip_next_fetch:-1, control_hazard:false}));

        let backend = Backend::new(
            cpu_config,
            Rc::clone(&instr_queue),
            Rc::clone(&memory_subsystem),
            Rc::clone(&arch_reg_file),
            Rc::clone(&frontend_control),
        );

        let frontend = Frontend::new(
            cpu_config,
            Rc::clone(&instr_queue),
            Rc::clone(&frontend_control));

        CPU {
            backend,
            frontend,
            memory_subsystem,
            arch_reg_file,
            cycle_cnt: 0,
            cycle_period: Duration::from_micros(1_000_000 / cpu_config.frequency_hz),
        }
    }

    pub(crate) fn run(&mut self, program: &Rc<Program>) {
        self.frontend.init(program);

        self.memory_subsystem.borrow_mut().init(program);

        loop {
            self.cycle_cnt += 1;
            println!("=======================================================================");
            println!("Cycle {}", self.cycle_cnt);
            self.memory_subsystem.borrow_mut().do_cycle();
            self.backend.do_cycle();
            self.frontend.do_cycle();
            thread::sleep(self.cycle_period);
        }
    }
}

struct ArgRegEntry {
    pub(crate) value: WordType,
}

pub struct ArgRegFile {
    entries: Vec<ArgRegEntry>,
}

impl ArgRegFile {
    fn new(rs_count: u16) -> ArgRegFile {
        let mut array = Vec::with_capacity(rs_count as usize);
        for _ in 0..rs_count {
            array.push(ArgRegEntry { value: 0 });
        }

        ArgRegFile { entries: array }
    }

    pub fn get_value(&self, reg: RegisterType) -> WordType {
        return self.entries.get(reg as usize).unwrap().value;
    }

    pub fn set_value(&mut self, reg: RegisterType, value: WordType) {
        let arch_reg = self.entries.get_mut(reg as usize).unwrap();
        arch_reg.value = value;
    }
}
