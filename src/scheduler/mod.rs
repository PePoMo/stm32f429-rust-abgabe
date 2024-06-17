pub mod context_switch;
use core::mem::MaybeUninit;

use context_switch::Task;

pub const TASK_QUEUE_SIZE: u32 = 16;

#[derive(Clone, Copy)]
pub enum SchedulingStrategy {
    EDF,
    FixedPriority,
    RoundRobin
}

pub struct Scheduler<'a> {
    pub strategy: SchedulingStrategy,
    pub task_queue: MaybeUninit<[Task<'a>; TASK_QUEUE_SIZE as usize]>,
    pub num_tasks: u32,
    pub current_task_id: u32,
    pub next_task: *mut u32,  // Stackpointer der naechsten Task
    pub current_task: *mut u32  // Stackpounter der aktuellen Task
}

impl<'a> Scheduler<'a> {
    pub fn new(strategy: SchedulingStrategy) -> Scheduler<'a> {
        return Scheduler {
            strategy: strategy,
            task_queue: MaybeUninit::uninit(),
            num_tasks: 0,
            current_task_id: 0,
            next_task: 0 as *mut u32,  // mit Nullpointer initialisiert
            current_task: 0 as *mut u32  // mit Nullpointer initialisiert
        };
    }

    pub fn queue_task(mut self, task: Task<'a>) {
        if self.num_tasks >= TASK_QUEUE_SIZE as u32 { 
            panic!("Task queue full! Cannot queue task.");
        }

        unsafe { self.task_queue.assume_init()[self.num_tasks as usize] = task; }
        self.num_tasks = self.num_tasks + 1;
    }

    pub fn schedule(self) -> () {
        match self.strategy {
            SchedulingStrategy::EDF => { self.schedule_edf(); },
            SchedulingStrategy::FixedPriority => { self.schedule_fixed_priority(); },
            SchedulingStrategy::RoundRobin => { self.schedule_round_robin(); },
            _ => panic!("Invalid scheduling strategy!")
        }
    }

    fn schedule_edf(mut self) -> () {

    }

    fn schedule_fixed_priority(mut self) -> () {

    }

    fn schedule_round_robin(mut self) -> () {
        self.current_task_id = self.current_task_id + 1;
        if self.current_task_id >= self.num_tasks { self.current_task_id = 0; }
        unsafe { self.next_task = self.task_queue.assume_init()[self.current_task_id as usize].stack_pointer; }

        if self.next_task != self.current_task {
            cortex_m::peripheral::SCB::set_pendsv();
        }
    }
}