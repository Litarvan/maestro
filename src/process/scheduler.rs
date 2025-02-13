//! The role of the process scheduler is to interrupt the currently running process periodicaly
//! to switch to another process that is in running state. The interruption is fired by the PIT
//! on IDT0.
//!
//! A scheduler cycle is a period during which the scheduler iterates through every processes.
//! The scheduler works by assigning a number of quantum for each process, based on the number of
//! running processes and their priority.
//! This number represents the number of ticks during which the process keeps running until
//! switching to the next process.

use core::cmp::max;
use core::ffi::c_void;
use crate::errno::Errno;
use crate::event::CallbackHook;
use crate::event;
use crate::idt::pic;
use crate::memory::malloc;
use crate::memory::stack;
use crate::memory;
use crate::process::Process;
use crate::process::pid::Pid;
use crate::process::regs::Regs;
use crate::process;
use crate::util::container::map::Map;
use crate::util::container::map::MapIterator;
use crate::util::container::map::TraversalType;
use crate::util::container::vec::Vec;
use crate::util::lock::*;
use crate::util::math;
use crate::util::ptr::IntSharedPtr;

/// The size of the temporary stack for context switching.
const TMP_STACK_SIZE: usize = 16 * memory::PAGE_SIZE;
/// The number of quanta for the process with the average priority.
const AVERAGE_PRIORITY_QUANTA: usize = 10;
/// The number of quanta for the process with the maximum priority.
const MAX_PRIORITY_QUANTA: usize = 30;

/// The structure representing the process scheduler.
pub struct Scheduler {
	/// A vector containing the temporary stacks for each CPU cores.
	tmp_stacks: Vec<malloc::Alloc<u8>>,

	/// The ticking callback hook, called at a regular interval to make the scheduler work.
	tick_callback_hook: CallbackHook,
	/// The total number of ticks since the instanciation of the scheduler.
	total_ticks: u64,

	/// A binary tree containing all processes registered to the current scheduler.
	processes: Map<Pid, IntSharedPtr<Process>>,
	/// The currently running process with its PID.
	curr_proc: Option<(Pid, IntSharedPtr<Process>)>,

	/// The sum of all priorities, used to compute the average priority.
	priority_sum: usize,
	/// The priority of the processs which has the current highest priority.
	priority_max: usize,
}

impl Scheduler {
	/// Creates a new instance of scheduler.
	pub fn new(cores_count: usize) -> Result<IntSharedPtr<Self>, Errno> {
		let mut tmp_stacks = Vec::new();
		for _ in 0..cores_count {
			tmp_stacks.push(malloc::Alloc::new_default(TMP_STACK_SIZE)?)?;
		}

		let callback = | _id: u32, _code: u32, regs: &Regs, ring: u32 | {
			Scheduler::tick(process::get_scheduler(), regs, ring);
		};
		let tick_callback_hook = event::register_callback(0x20, 0, callback)?;

		IntSharedPtr::new(Self {
			tmp_stacks,

			tick_callback_hook,
			total_ticks: 0,

			processes: Map::new(),
			curr_proc: None,

			priority_sum: 0,
			priority_max: 0,
		})
	}

	/// Returns a pointer to the top of the tmp stack for the given core `core`.
	pub fn get_tmp_stack(&mut self, core: u32) -> *mut c_void {
		unsafe {
			self.tmp_stacks[core as usize].as_ptr_mut().add(TMP_STACK_SIZE) as *mut _
		}
	}

	/// Returns the number of processes registered on the scheduler.
	pub fn get_processes_count(&self) -> usize {
		self.processes.count()
	}

	/// Calls the given function `f` for each processes.
	pub fn foreach_process<F: FnMut(&Pid, &mut IntSharedPtr<Process>)>(&mut self, f: F) {
		self.processes.foreach_mut(f, TraversalType::InOrder);
	}

	/// Returns the process with PID `pid`. If the process doesn't exist, the function returns
	/// None.
	pub fn get_by_pid(&self, pid: Pid) -> Option<IntSharedPtr<Process>> {
		Some(self.processes.get(pid)?.clone())
	}

	/// Returns the process with TID `tid`. If the process doesn't exist, the function returns
	/// None.
	pub fn get_by_tid(&self, _tid: Pid) -> Option<IntSharedPtr<Process>> {
		// TODO
		todo!();
	}

	/// Returns the current running process. If no process is running, the function returns None.
	pub fn get_current_process(&mut self) -> Option<IntSharedPtr<Process>> {
		Some(self.curr_proc.as_ref().cloned()?.1)
	}

	/// Updates the scheduler's heuristic with the new priority of a process.
	/// `old` is the old priority of the process.
	/// `new` is the new priority of the process.
	/// The function doesn't need to know the process which has been updated since it updates
	/// global informations.
	pub fn update_priority(&mut self, old: usize, new: usize) {
		self.priority_sum = self.priority_sum - old + new;

		if new >= self.priority_max {
			self.priority_max = new;
		}

		// FIXME: Unable to determine priority_max when new < old
	}

	/// Adds a process to the scheduler.
	pub fn add_process(&mut self, process: Process) -> Result<IntSharedPtr<Process>, Errno> {
		let pid = process.get_pid();
		let priority = process.get_priority();
		let ptr = IntSharedPtr::new(process)?;
		self.processes.insert(pid, ptr.clone())?;
		self.update_priority(0, priority);

		Ok(ptr)
	}

	/// Removes the process with the given pid `pid`.
	pub fn remove_process(&mut self, pid: Pid) {
		if let Some(proc_mutex) = self.get_by_pid(pid) {
			let guard = proc_mutex.lock();
			let process = guard.get();

			let priority = process.get_priority();
			self.processes.remove(pid);
			self.update_priority(priority, 0);
		}
	}

	// TODO Clean
	/// Returns the average priority of a process.
	/// `priority_sum` is the sum of all processes' priorities.
	/// `processes_count` is the number of processes.
	fn get_average_priority(priority_sum: usize, processes_count: usize) -> usize {
		priority_sum / processes_count
	}

	// TODO Clean
	/// Returns the number of quantum for the given priority.
	/// `priority` is the process's priority.
	/// `priority_sum` is the sum of all processes' priorities.
	/// `priority_max` is the highest priority a process currently has.
	/// `processes_count` is the number of processes.
	fn get_quantum_count(priority: usize, priority_sum: usize, priority_max: usize,
		processes_count: usize) -> usize {
		let n = math::integer_linear_interpolation::<isize>(priority as _,
			Self::get_average_priority(priority_sum, processes_count) as _,
			priority_max as _,
			AVERAGE_PRIORITY_QUANTA as _,
			MAX_PRIORITY_QUANTA as _);
		max(1, n) as _
	}

	// TODO Clean
	/// Tells whether the given process `process` can run.
	fn can_run(process: &Process, _priority_sum: usize, _priority_max: usize,
		_processes_count: usize) -> bool {
		if process.can_run() {
			// TODO fix
			//process.quantum_count < Self::get_quantum_count(process.get_priority(), priority_sum,
			//	priority_max, processes_count)
			true
		} else {
			false
		}
	}

	// TODO Clean
	/// Returns the next process to run with its PID. If the process is changed, the quantum count
	/// of the previous process is reset.
	fn get_next_process(&self) -> Option<(Pid, IntSharedPtr<Process>)> {
		let priority_sum = self.priority_sum;
		let priority_max = self.priority_max;
		let processes_count = self.processes.count();
		// If no process exist, nothing to run
		if processes_count == 0 {
			return None;
		}

		// Getting the current process, or take the first process in the list if no process is
		// running
		let (curr_pid, curr_proc) = self.curr_proc.clone().or_else(|| {
			let (pid, proc) = self.processes.get_min(0)?;
			Some((*pid, proc.clone()))
		})?;

		// Closure iterating the tree to find an available process
		let next = | iter: MapIterator<Pid, IntSharedPtr<Process>> | {
			let mut proc: Option<(Pid, IntSharedPtr<Process>)> = None;

			// Iterating over processes
			for (pid, process) in iter {
				let runnable = {
					let guard = process.lock();
					Self::can_run(guard.get(), priority_sum, priority_max, processes_count)
				};

				// FIXME Potenial race condition? (checking if runnable, then unlocking and using
				// the result of the check)
				if runnable {
					proc = Some((*pid, process.clone()));
					break;
				}
			}

			proc
		};

		let mut iter = self.processes.iter();
		// Setting the iterator next to the current running process
		iter.jump(&curr_pid);
		iter.next();

		// Running the loop to reach the end of processes list
		let mut next_proc = next(iter);
		// If no suitable process is found, going back to the beginning to check the processes
		// located before the previous process
		if next_proc.is_none() {
			next_proc = next(self.processes.iter());
		}

		let (next_pid, next_proc) = next_proc?;

		if next_pid != curr_pid || processes_count == 1 {
			curr_proc.lock().get_mut().quantum_count = 0;
		}
		Some((next_pid, next_proc))
	}

	/// Ticking the scheduler. This function saves the data of the currently running process, then
	/// switches to the next process to run.
	/// `mutex` is the scheduler's mutex.
	/// `regs` is the state of the registers from the paused context.
	/// `ring` is the ring of the paused context.
	fn tick(mutex: &mut IntMutex<Self>, regs: &Regs, ring: u32) -> ! {
		// Disabling interrupts to avoid getting one right after unlocking mutexes
		cli!();

		let mut guard = mutex.lock();
		let scheduler = guard.get_mut();

		scheduler.total_ticks += 1;

		// If a process is running, save its registers
		if let Some(curr_proc) = scheduler.get_current_process() {
			let mut guard = curr_proc.lock();
			let curr_proc = guard.get_mut();

			curr_proc.regs = *regs;
			curr_proc.syscalling = ring < 3;
		}

		// The current core ID
		let core_id = 0; // TODO
		// Getting the temporary stack
		let tmp_stack = scheduler.get_tmp_stack(core_id);

		if let Some(next_proc) = scheduler.get_next_process() {
			// Set the process as current
			scheduler.curr_proc = Some(next_proc.clone());

			drop(guard);
			unsafe {
				event::unlock_callbacks(0x20);
			}
			pic::end_of_interrupt(0x0);

			unsafe {
				stack::switch(Some(tmp_stack), move || {
					let (syscalling, regs) = {
						let mut guard = next_proc.1.lock();
						let proc = guard.get_mut();

						proc.prepare_switch();
						(proc.is_syscalling(), proc.regs)
					};

					drop(next_proc);

					// Resuming execution
					regs.switch(!syscalling);
				}).unwrap();
			}

			unreachable!();
		} else {
			drop(guard);
			unsafe {
				event::unlock_callbacks(0x20);
			}
			pic::end_of_interrupt(0x0);

			unsafe {
				crate::loop_reset(tmp_stack);
			}
		}
	}

	/// Returns the total number of ticks since the instanciation of the scheduler.
	pub fn get_total_ticks(&self) -> u64 {
		self.total_ticks
	}
}
