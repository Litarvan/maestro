//! The `clone` system call creates a child process.

use core::ffi::c_void;
use crate::errno::Errno;
use crate::process::ForkOptions;
use crate::process::Process;
use crate::process::mem_space::ptr::SyscallPtr;
use crate::process::regs::Regs;
use crate::process::user_desc::UserDesc;

/// TODO doc
const CLONE_IO: i32 = -0x80000000;
/// If specified, the parent and child processes share the same memory space.
const CLONE_VM: i32 = 0x100;
/// TODO doc
const CLONE_FS: i32 = 0x200;
/// If specified, the parent and child processes share the same file descriptors table.
const CLONE_FILES: i32 = 0x400;
/// If specified, the parent and child processes share the same signal handlers table.
const CLONE_SIGHAND: i32 = 0x800;
/// TODO doc
const CLONE_PIDFD: i32 = 0x1000;
/// TODO doc
const CLONE_PTRACE: i32 = 0x2000;
/// TODO doc
const CLONE_VFORK: i32 = 0x4000;
/// TODO doc
const CLONE_PARENT: i32 = 0x8000;
/// TODO doc
const CLONE_THREAD: i32 = 0x10000;
/// TODO doc
const CLONE_NEWNS: i32 = 0x20000;
/// TODO doc
const CLONE_SYSVSEM: i32 = 0x40000;
/// TODO doc
const CLONE_SETTLS: i32 = 0x80000;
/// TODO doc
const CLONE_PARENT_SETTID: i32 = 0x100000;
/// TODO doc
const CLONE_CHILD_CLEARTID: i32 = 0x200000;
/// TODO doc
const CLONE_DETACHED: i32 = 0x400000;
/// TODO doc
const CLONE_UNTRACED: i32 = 0x800000;
/// TODO doc
const CLONE_CHILD_SETTID: i32 = 0x1000000;
/// TODO doc
const CLONE_NEWCGROUP: i32 = 0x2000000;
/// TODO doc
const CLONE_NEWUTS: i32 = 0x4000000;
/// TODO doc
const CLONE_NEWIPC: i32 = 0x8000000;
/// TODO doc
const CLONE_NEWUSER: i32 = 0x10000000;
/// TODO doc
const CLONE_NEWPID: i32 = 0x20000000;
/// TODO doc
const CLONE_NEWNET: i32 = 0x40000000;

/// The implementation of the `clone` syscall.
pub fn clone(regs: &Regs) -> Result<i32, Errno> {
	let flags = regs.ebx as i32;
	let stack = regs.ecx as *mut c_void;
	let _parent_tid: SyscallPtr<i32> = (regs.edx as usize).into();
	let tls = regs.esi as i32;
	let _child_tid: SyscallPtr<i32> = (regs.edi as usize).into();

	let new_tid = {
		// The current process
		let curr_mutex = Process::get_current().unwrap();
		// A weak pointer to the new process's parent
		let parent = curr_mutex.new_weak();

		let mut curr_guard = curr_mutex.lock();
		let curr_proc = curr_guard.get_mut();

		if flags & CLONE_PARENT_SETTID != 0 {
			// TODO
			todo!();
		}

		let fork_options = ForkOptions {
			share_memory: flags & CLONE_VM != 0,
			share_fd: flags & CLONE_FILES != 0,
			share_sighand: flags & CLONE_SIGHAND != 0,

			vfork: flags & CLONE_VFORK != 0,
		};
		let new_mutex = curr_proc.fork(parent, fork_options)?;
		let mut new_guard = new_mutex.lock();
		let new_proc = new_guard.get_mut();

		// Setting the process's registers
		let mut new_regs = regs.clone();
		// Setting return value to `0`
		new_regs.eax = 0;
		// Setting stack
		new_regs.esp = if stack.is_null() {
			regs.esp as _
		} else {
			stack as _
		};
		// Setting TLS
		if flags & CLONE_SETTLS != 0 {
			let _tls: SyscallPtr<UserDesc> = (tls as usize).into();

			// TODO
			todo!();
		}
		new_proc.set_regs(new_regs);

		if flags & CLONE_CHILD_CLEARTID != 0 {
			// TODO new_proc.set_clear_child_tid(child_tid);
			todo!();
		}
		if flags & CLONE_CHILD_SETTID != 0 {
			// TODO
			todo!();
		}

		new_proc.get_tid()
	};

	if flags & CLONE_VFORK != 0 {
		// Letting another process run instead of the current. Because the current process must now
		// wait for the child process to terminate or execute a program
		crate::wait();
	}

	Ok(new_tid as _)
}
