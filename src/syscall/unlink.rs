//! The `unlink` system call deletes the given file from its filesystem. If no link remain to the
//! inode, the function also removes the inode.

use crate::errno::Errno;
use crate::file::fcache;
use crate::file::path::Path;
use crate::process::Process;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::regs::Regs;
use crate::util::FailableClone;

/// The implementation of the `unlink` syscall.
pub fn unlink(regs: &Regs) -> Result<i32, Errno> {
	let pathname: SyscallString = (regs.ebx as usize).into();

	let (path, uid, gid) = {
		// Getting the process
		let mutex = Process::get_current().unwrap();
		let mut guard = mutex.lock();
		let proc = guard.get_mut();

		let mem_space = proc.get_mem_space().unwrap();
		let mem_space_guard = mem_space.lock();
		let path = Path::from_str(pathname.get(&mem_space_guard)?.ok_or(errno!(EFAULT))?, true)?;
		(path, proc.get_euid(), proc.get_egid())
	};

	// Getting the path of the parent directory
	let mut parent_path = path.failable_clone()?;
	parent_path.pop();

	// TODO If the file is busy, remove only when the last fd to it is closed

	// Removing the file
	{
		let mutex = fcache::get();
		let mut guard = mutex.lock();
		let files_cache = guard.get_mut().as_mut().unwrap();

		// Getting file
		let file_mutex = files_cache.get_file_from_path(&path, uid, gid, true)?;
		let mut file_guard = file_mutex.lock();
		let file = file_guard.get_mut();

		files_cache.remove_file(file, uid, gid)?;
	}

	Ok(0)
}
