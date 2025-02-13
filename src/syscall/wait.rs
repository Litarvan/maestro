//! The `wait` system call is a simpler version of the `waitpid` system call.

use crate::errno::Errno;
use crate::process::mem_space::ptr::SyscallPtr;
use crate::process::regs::Regs;
use super::waitpid;

/// The implementation of the `wait` syscall.
pub fn wait(regs: &Regs) -> Result<i32, Errno> {
	let wstatus: SyscallPtr<i32> = (regs.ebx as usize).into();
	waitpid::do_waitpid(-1, wstatus, 0, None)
}
