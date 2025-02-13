//! `select` waits for a file descriptor in the given sets to be readable, writable or for an
//! exception to occur.

use core::cmp::min;
use core::mem::size_of;
use crate::errno::Errno;
use crate::process::Process;
use crate::process::mem_space::ptr::SyscallPtr;
use crate::process::mem_space::ptr::SyscallSlice;
use crate::syscall::Regs;
use crate::time::unit::TimeUnit;
use crate::time::unit::Timeval;
use crate::time;
use crate::types::*;

/// The number of file descriptors in FDSet.
pub const FD_SETSIZE: usize = 1024;

/// Structure representing `fd_set`.
#[repr(C)]
pub struct FDSet {
	/// The set's bitfield.
	fds_bits: [c_long; FD_SETSIZE / (8 * size_of::<c_long>())],
}

impl FDSet {
	/// Tells whether the given file descriptor `fd` is set in the list.
	pub fn is_set(&self, fd: u32) -> bool {
		if fd as usize >= FD_SETSIZE {
			return false;
		}

		// TODO Check correctness
		self.fds_bits[(fd as usize) / (8 * size_of::<c_long>())]
			>> (fd % ((8 * size_of::<c_long>()) as u32)) != 0
	}

	/// Sets the bit for file descriptor `fd`.
	pub fn set(&mut self, fd: u32) {
		// TODO Check correctness
		self.fds_bits[(fd as usize) / (8 * size_of::<c_long>())]
			|= 1 << (fd % ((8 * size_of::<c_long>()) as u32));
	}

	/// Clears the bit for file descriptor `fd`.
	pub fn clear(&mut self, fd: u32) {
		// TODO Check correctness
		self.fds_bits[(fd as usize) / (8 * size_of::<c_long>())]
			&= !(1 << (fd % ((8 * size_of::<c_long>()) as u32)));
	}
}

/// TODO doc
pub fn do_select<T: TimeUnit>(nfds: u32,
	readfds: SyscallPtr<FDSet>,
	writefds: SyscallPtr<FDSet>,
	exceptfds: SyscallPtr<FDSet>,
	timeout: SyscallPtr<T>,
	_sigmask: Option<SyscallSlice<u8>>
) -> Result<i32, Errno> {
	// Getting start timestamp
	let start = time::get_struct::<T>(b"TODO").unwrap(); // TODO Select a clock

	// Getting timeout
	let timeout = {
		let proc_mutex = Process::get_current().unwrap();
		let proc_guard = proc_mutex.lock();
		let proc = proc_guard.get();

		let mem_space = proc.get_mem_space().unwrap();
		let mem_space_guard = mem_space.lock();
		timeout.get(&mem_space_guard)?.map(| t | t.clone()).unwrap_or_default()
	};

	// The end timestamp
	let end = start + timeout;

	loop {
		let mut events_count = 0;

		{
			for fd_id in 0..min(nfds as u32, FD_SETSIZE as u32) {
				let (mem_space, fd) = {
					let proc_mutex = Process::get_current().unwrap();
					let proc_guard = proc_mutex.lock();
					let proc = proc_guard.get();

					let mem_space = proc.get_mem_space().unwrap();
					let fd = proc.get_fd(fd_id);
					(mem_space, fd)
				};

				if let Some(fd) = fd {
					let open_file_mutex = fd.get_open_file();
					let open_file_guard = open_file_mutex.lock();
					let open_file = open_file_guard.get();

					let mem_space_guard = mem_space.lock();

					if let Some(readfds) = readfds.get_mut(&mem_space_guard)? {
						if readfds.is_set(fd_id) {
							// TODO
							if true || open_file.eof() {
								readfds.set(fd_id);
								events_count += 1;
							} else {
								readfds.clear(fd_id);
							}
						}
					}

					if let Some(writefds) = writefds.get_mut(&mem_space_guard)? {
						if writefds.is_set(fd_id) {
							// TODO
							if true {
								writefds.set(fd_id);
								events_count += 1;
							} else {
								writefds.clear(fd_id);
							}
						}
					}

					if let Some(exceptfds) = exceptfds.get_mut(&mem_space_guard)? {
						if exceptfds.is_set(fd_id) {
							// TODO
							if false {
								//exceptfds.set(fd_id);
								//events_count += 1;
							} else {
								exceptfds.clear(fd_id);
							}
						}
					}
				} else {
					let mem_space_guard = mem_space.lock();

					let read = readfds.get_mut(&mem_space_guard)?
						.map(| fds | fds.is_set(fd_id)).unwrap_or(false);
					let write = writefds.get_mut(&mem_space_guard)?
						.map(| fds | fds.is_set(fd_id)).unwrap_or(false);
					let except = exceptfds.get_mut(&mem_space_guard)?
						.map(| fds | fds.is_set(fd_id)).unwrap_or(false);

					if read || write || except {
						return Err(errno!(EBADF));
					}
				}
			}
		}

		// If one or more events occured, return
		if events_count > 0 {
			return Ok(events_count);
		}

		// TODO Select a clock
		let curr = time::get_struct::<T>(b"TODO").unwrap();
		// On timeout, return 0
		if curr >= end {
			return Ok(0);
		}

		// TODO Make the process sleep?
		crate::wait();
	}
}

/// TODO doc
pub fn select(regs: &Regs) -> Result<i32, Errno> {
	let nfds = regs.ebx as c_int;
	let readfds: SyscallPtr<FDSet> = (regs.ecx as usize).into();
	let writefds: SyscallPtr<FDSet> = (regs.edx as usize).into();
	let exceptfds: SyscallPtr<FDSet> = (regs.esi as usize).into();
	let timeout: SyscallPtr<Timeval> = (regs.edi as usize).into();

	do_select(nfds as _, readfds, writefds, exceptfds, timeout, None)
}
