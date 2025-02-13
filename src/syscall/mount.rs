//! The mount system call allows to mount a filesystem on the system.

use core::ffi::c_void;
use crate::errno::Errno;
use crate::errno;
use crate::file::FileType;
use crate::file::fcache;
use crate::file::fs;
use crate::file::mountpoint::MountPoint;
use crate::file::mountpoint::MountSource;
use crate::file::mountpoint;
use crate::file::path::Path;
use crate::process::Process;
use crate::process::mem_space::ptr::SyscallPtr;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::regs::Regs;

/// The implementation of the `mount` syscall.
pub fn mount(regs: &Regs) -> Result<i32, Errno> {
	let source: SyscallString = (regs.ebx as usize).into();
	let target: SyscallString = (regs.ecx as usize).into();
	let filesystemtype: SyscallString = (regs.edx as usize).into();
	let mountflags = regs.esi as u32;
	let _data: SyscallPtr<c_void> = (regs.edi as usize).into();

	// Getting the process
	let mutex = Process::get_current().unwrap();
	let guard = mutex.lock();
	let proc = guard.get();

	let mem_space = proc.get_mem_space().unwrap();
	let mem_space_guard = mem_space.lock();

	// Getting strings
	let source_slice = source.get(&mem_space_guard)?.ok_or(errno!(EFAULT))?;
	let target_slice = target.get(&mem_space_guard)?.ok_or(errno!(EFAULT))?;
	let filesystemtype_slice = filesystemtype.get(&mem_space_guard)?.ok_or(errno!(EFAULT))?;

	// Getting the mount source
	let mount_source = MountSource::from_str(source_slice)?;

	// Getting the target file
	let target_path = Path::from_str(target_slice, true)?;
	let target_mutex = {
		let mut guard = fcache::get().lock();
		let fcache = guard.get_mut().as_mut().unwrap();

		fcache.get_file_from_path(&target_path, proc.get_euid(), proc.get_egid(), true)?
	};
	let target_guard = target_mutex.lock();
	let target_file = target_guard.get();

	// Checking the target is a directory
	if target_file.get_file_type() != FileType::Directory {
		return Err(errno!(ENOTDIR));
	}

	// TODO Check for loop between source and target

	let fs_type = fs::get_fs(filesystemtype_slice).ok_or(errno!(ENODEV))?;

	// TODO Use `data`
	// Creating mountpoint
	let mount = MountPoint::new(mount_source, Some(fs_type), mountflags, target_path)?;
	mountpoint::register(mount)?;

	Ok(0)
}
