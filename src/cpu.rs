use std::cell::RefCell;
use std::error::Error;
use std::fs::File;
use std::rc::Rc;
use std::thread;
use std::time::Duration;

use serde::Deserialize;

use crate::backend::backend::Backend;
use crate::frontend::frontend::{Frontend, FrontendControl};
use crate::instructions::instructions::{DWordType, InstrQueue, Program, RegisterType};
use crate::memory_subsystem::memory_subsystem::MemorySubsystem;

pub struct PerfCounters {
    pub branch_miss_prediction_cnt: u64,
    pub branch_good_predictions_cnt: u64,
    pub decode_cnt: u64,
    pub issue_cnt: u64,
    pub dispatch_cnt: u64,
    pub execute_cnt: u64,
    pub retired_cnt: u64,
    pub cycle_cnt: u64,
    pub bad_speculation_cnt: u64,
    pub pipeline_flushes: u64,
}


impl PerfCounters {
    pub fn new() -> Self {
        Self {
            decode_cnt: 0,
            issue_cnt: 0,
            dispatch_cnt: 0,
            execute_cnt: 0,
            retired_cnt: 0,
            cycle_cnt: 0,
            bad_speculation_cnt: 0,
            branch_miss_prediction_cnt: 0,
            branch_good_predictions_cnt: 0,
            pipeline_flushes: 0,
        }
    }
}

#[derive(Clone, Deserialize, Debug)]
pub struct Trace {
    pub decode: bool,
    pub issue: bool,
    pub allocate_rs: bool,
    pub dispatch: bool,
    pub execute: bool,
    pub retire: bool,
    pub cycle: bool,
    pub pipeline_flush: bool,
}

impl Default for Trace {
    fn default() -> Self {
        Trace {
            decode: false,
            issue: false,
            allocate_rs: false,
            dispatch: false,
            execute: false,
            retire: false,
            cycle: false,
            pipeline_flush: false,
        }
    }
}

#[derive(Clone, Deserialize, Debug)]
pub struct CPUConfig {
    // the number of physical registers
    pub phys_reg_count: u16,
    // the number of instructions the frontend can fetch/decode per clock cycle.
    pub frontend_n_wide: u8,
    // the size of the instruction queue between frontend and backend
    pub instr_queue_capacity: u16,
    // the frequency of the CPU in Hz.
    pub frequency_hz: u64,
    // the number of reservation stations
    pub rs_count: u16,
    // the size of the memory in machine words
    pub memory_size: u32,
    // the capacity of the store buffer
    pub sb_capacity: u16,
    // the number of line fill buffers; currently there are no line fill buffer
    // it is just a limit of the number of stores that can commit to memory
    // per clock cycle (there is also no cache)
    pub lfb_count: u8,
    // the capacity of the reorder buffer
    pub rob_capacity: u16,
    // the number of execution units
    pub eu_count: u8,
    // if processing of a single instruction should be traced (printed)
    pub trace: Trace,
    // the number of instructions that can retire per clock cycle
    pub retire_n_wide: u8,
    // the number of instructions that can be dispatched (send to execution units) every clock cycle.
    pub dispatch_n_wide: u8,
    // the number of instructions that can be issued to  the rob or finding reservation stations, every clock cycle.
    pub issue_n_wide: u8,
}

impl Default for CPUConfig {
    fn default() -> Self {
        CPUConfig {
            phys_reg_count: 64,
            frontend_n_wide: 4,
            instr_queue_capacity: 64,
            frequency_hz: 4,
            rs_count: 64,
            memory_size: 128,
            sb_capacity: 16,
            lfb_count: 4,
            rob_capacity: 32,
            eu_count: 10,
            trace: Trace::default(),
            retire_n_wide: 4,
            dispatch_n_wide: 4,
            issue_n_wide: 4,
        }
    }
}

pub fn load_cpu_config(file_path: &str) -> Result<CPUConfig, Box<dyn Error>> {
    let file = File::open(file_path)?;
    let config = serde_yaml::from_reader(file)?;
    Ok(config)
}

pub struct CPU {
    pub(crate) backend: Backend,
    pub(crate) frontend: Frontend,
    pub(crate) memory_subsystem: Rc<RefCell<MemorySubsystem>>,
    pub(crate) arch_reg_file: Rc<RefCell<ArgRegFile>>,
    pub(crate) cycle_period: Duration,
    pub(crate) trace: Trace,
    pub(crate) perf_counters: Rc<RefCell<PerfCounters>>,
}

impl CPU {
    pub fn new(cpu_config: &CPUConfig) -> CPU {
        let instr_queue = Rc::new(RefCell::new(InstrQueue::new(cpu_config.instr_queue_capacity)));

        let perf_counters = Rc::new(RefCell::new(PerfCounters::new()));

        let memory_subsystem = Rc::new(RefCell::new(
            MemorySubsystem::new(cpu_config)));

        let arch_reg_file = Rc::new(RefCell::new(
            ArgRegFile::new(GENERAL_ARG_REG_CNT + SPECIAL_ARG_REG_CNT)));

        // on ARM the stack grows down (from larger address to smaller address)
        arch_reg_file.borrow_mut().set_value(SP, cpu_config.memory_size as DWordType);

        let frontend_control = Rc::new(RefCell::new(
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

    pub fn run(&mut self, program: &Rc<Program>) {
        self.frontend.init(program);

        self.memory_subsystem.borrow_mut().init(program);

        while !self.backend.exit {
            self.perf_counters.borrow_mut().cycle_cnt += 1;

            if self.trace.cycle {
                self.trace_cycle();
            }
            self.memory_subsystem.borrow_mut().do_cycle();
            self.backend.do_cycle();
            self.frontend.do_cycle();
            thread::sleep(self.cycle_period);
        }

        loop {
            if self.memory_subsystem.borrow_mut().sb.is_empty() {
                break;
            }

            self.memory_subsystem.borrow_mut().do_cycle();
        }

        println!("Program complete!");
    }

    fn trace_cycle(&mut self) {
        let perf_counters = self.perf_counters.borrow_mut();
        let branch_total = perf_counters.branch_miss_prediction_cnt + perf_counters.branch_good_predictions_cnt;

        let ipc = perf_counters.retired_cnt as f32 / perf_counters.cycle_cnt as f32;

        let branch_prediction = if branch_total != 0 {
            100.0 * perf_counters.branch_good_predictions_cnt as f32 / branch_total as f32
        } else {
            0.0
        };

        let mut message = String::new();

        message.push_str(&format!("[Cycles:{}]", perf_counters.cycle_cnt));
        message.push_str(&format!("[IPC={:.2}]", ipc));
        message.push_str(&format!("[Decoded={}]", perf_counters.decode_cnt));
        message.push_str(&format!("[Issued={}]", perf_counters.issue_cnt));
        message.push_str(&format!("[Dispatched={}]", perf_counters.dispatch_cnt));
        message.push_str(&format!("[Executed={}]", perf_counters.execute_cnt));
        message.push_str(&format!("[Retired={}]", perf_counters.retired_cnt));
        message.push_str(&format!("[Branch Tot={}, Pred={:.2}%]", branch_total, branch_prediction));
        message.push_str(&format!("[Pipeline Flush={}]", perf_counters.pipeline_flushes));

        println!("{}", message);
    }
}

pub const GENERAL_ARG_REG_CNT: u16 = 31;
pub const SPECIAL_ARG_REG_CNT: u16 = 1;
pub const FP: u16 = 11;
pub const SP: u16 = 13;
pub const LR: u16 = 14;
pub const PC: u16 = 15;
pub const CPSR: u16 = GENERAL_ARG_REG_CNT;

pub const ZERO_FLAG: u8 = 30;
pub const NEGATIVE_FLAG: u8 = 31;
pub const CARRY_FLAG: u8 = 29;
pub const OVERFLOW_FLAG: u8 = 28;

struct ArgRegEntry {
    value: DWordType,
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

    pub fn get_value(&self, reg: RegisterType) -> DWordType {
        let entry = self.entries.get(reg as usize).unwrap();
        return entry.value;
    }

    pub fn set_value(&mut self, reg: RegisterType, value: DWordType) {
        let entry = self.entries.get_mut(reg as usize).unwrap();
        entry.value = value;
    }
}
