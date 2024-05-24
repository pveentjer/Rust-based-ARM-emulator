use std::cell::RefCell;
use std::error::Error;
use std::fs::File;
use std::rc::Rc;
use std::thread;
use std::time::Duration;
use serde::Deserialize;

use crate::backend::backend::Backend;
use crate::frontend::frontend::{Frontend, FrontendControl};
use crate::instructions::instructions::{InstrQueue, Program, RegisterType, WordType};
use crate::memory_subsystem::memory_subsystem::MemorySubsystem;


pub(crate) struct PerfCounters {
    pub decode_cnt: u64,
    pub issue_cnt: u64,
    pub dispatch_cnt: u64,
    pub execute_cnt: u64,
    pub retire_cnt: u64,
    pub cycle_cnt: u64,
}

impl PerfCounters {
    pub fn new() -> Self {
        Self { decode_cnt: 0, issue_cnt: 0, dispatch_cnt: 0, execute_cnt: 0, retire_cnt: 0, cycle_cnt: 0 }
    }
}

#[derive(Clone, Deserialize,Debug)]
pub(crate) struct Trace {
    pub decode: bool,
    pub issue: bool,
    pub dispatch: bool,
    pub execute: bool,
    pub retire: bool,
    pub cycle: bool,
}

#[derive(Clone, Deserialize,Debug)]
pub(crate) struct CPUConfig {
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
    pub(crate) trace: Trace,
    // the number of instructions that can retire per clock cycle
    pub(crate) retire_n_wide: u8,
    // the number of instructions that can be dispatched (send to execution units) every clock cycle.
    pub(crate) dispatch_n_wide: u8,
    // the number of instructions that can be issued to  the rob or finding reservation stations, every clock cycle.
    pub(crate) issue_n_wide: u8,
    // The size of the stack
    pub(crate) stack_capacity: u32,
}

pub fn load_cpu_config(file_path: &str) -> Result<CPUConfig, Box<dyn Error>> {
    let file = File::open(file_path)?;
    let config = serde_yaml::from_reader(file)?;
    Ok(config)
}


pub(crate) struct CPU {
    backend: Backend,
    frontend: Frontend,
    memory_subsystem: Rc<RefCell<MemorySubsystem>>,
    arch_reg_file: Rc<RefCell<ArgRegFile>>,
    cycle_period: Duration,
    trace: Trace,
    perf_counters: Rc<RefCell<PerfCounters>>,
}

impl CPU {
    pub(crate) fn new(cpu_config: &CPUConfig) -> CPU {
        let instr_queue = Rc::new(RefCell::new(InstrQueue::new(cpu_config.instr_queue_capacity)));

        let perf_counters = Rc::new(RefCell::new(PerfCounters::new()));

        let memory_subsystem = Rc::new(RefCell::new(
            MemorySubsystem::new(cpu_config)));

        let arch_reg_file = Rc::new(RefCell::new(
            ArgRegFile::new(GENERAL_ARG_REG_CNT)));

        // on ARM the stack grows down (from larger address to smaller address)
        arch_reg_file.borrow_mut().set_value(SP, cpu_config.memory_size as WordType);

        let mut frontend_control = Rc::new(RefCell::new(
            FrontendControl { halted: false }));

        let backend = Backend::new(
            cpu_config,
            Rc::clone(&instr_queue),
            Rc::clone(&memory_subsystem),
            Rc::clone(&arch_reg_file),
            Rc::clone(&frontend_control),
            Rc::clone(&perf_counters),
        );

        let frontend = Frontend::new(
            cpu_config,
            Rc::clone(&instr_queue),
            Rc::clone(&frontend_control),
            Rc::clone(&perf_counters),
            Rc::clone(&arch_reg_file),
        );


        let x = Duration::from_micros(1_000_000 / cpu_config.frequency_hz);
        println!("Duration: {:?}", x);

        CPU {
            backend,
            frontend,
            memory_subsystem,
            arch_reg_file,
            cycle_period: Duration::from_micros(1_000_000 / cpu_config.frequency_hz),
            trace: cpu_config.trace.clone(),
            perf_counters: Rc::clone(&perf_counters),
        }
    }

    pub(crate) fn run(&mut self, program: &Rc<Program>) {
        self.frontend.init(program);

        self.memory_subsystem.borrow_mut().init(program);

        while !self.backend.exit {
            self.perf_counters.borrow_mut().cycle_cnt += 1;

            if self.trace.cycle {
                let perf_counters = self.perf_counters.borrow_mut();
                println!("[Cycles:{}][Decoded={}][Issued={}][Dispatched={}][Executed={}][Retired={}][IPC={:.2}]",
                         perf_counters.cycle_cnt,
                         perf_counters.decode_cnt,
                         perf_counters.issue_cnt,
                         perf_counters.dispatch_cnt,
                         perf_counters.execute_cnt,
                         perf_counters.retire_cnt,
                         perf_counters.retire_cnt as f32 / perf_counters.cycle_cnt as f32
                );
            }
            self.memory_subsystem.borrow_mut().do_cycle();
            self.backend.do_cycle();
            self.frontend.do_cycle();
            thread::sleep(self.cycle_period);
        }

        println!("Program complete!");
    }
}

pub const GENERAL_ARG_REG_CNT: u16 = 31;
pub const FP: u16 = 11;
pub const SP: u16 = 13;
pub const LR: u16 = 14;
pub const PC: u16 = 15;

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
