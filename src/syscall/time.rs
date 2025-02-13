//! The `time` syscall allows to retrieve the number of seconds elapsed since the UNIX Epoch.

use crate::errno::Errno;
use crate::process::Process;
use crate::process::mem_space::ptr::SyscallPtr;
use crate::process::regs::Regs;
use crate::time;

// TODO Watch for timestamp overflow

/// The implementation of the `time` syscall.
pub fn time(regs: &Regs) -> Result<i32, Errno> {
	let tloc: SyscallPtr<u32> = (regs.ebx as usize).into();

	let mutex = Process::get_current().unwrap();
	let mut guard = mutex.lock();
	let proc = guard.get_mut();

	let mem_space = proc.get_mem_space().unwrap();
	let mem_space_guard = mem_space.lock();

	// Getting the current timestamp
	let time = time::get().unwrap_or(0);

	// Writing the timestamp to the given location, if not null
	if let Some(tloc) = tloc.get_mut(&mem_space_guard)? {
		*tloc = time as _;
	}

	Ok(time as _)
}
