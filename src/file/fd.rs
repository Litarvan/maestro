//! This module implements file descriptors-related features.
//! A file descriptor is an ID held by a process pointing to an entry in the open file description
//! table.

use crate::errno::Errno;
use crate::file::open_file::OpenFile;
use crate::util::lock::Mutex;
use crate::util::ptr::SharedPtr;

/// The maximum number of file descriptors that can be open system-wide at once.
const TOTAL_MAX_FD: usize = 4294967295;

/// File descriptor flag: If set, the file descriptor is closed on successful call to `execve`.
pub const FD_CLOEXEC: i32 = 1;

/// The total number of file descriptors open system-wide.
static TOTAL_FD: Mutex<usize> = Mutex::new(0);

/// Increments the total number of file descriptors open system-wide.
/// If the maximum amount of file descriptors is reached, the function does nothing and returns an
/// error with the appropriate errno.
fn increment_total() -> Result<(), Errno> {
	let mut guard = TOTAL_FD.lock();

	if *guard.get() >= TOTAL_MAX_FD {
		return Err(errno!(ENFILE));
	}
	*guard.get_mut() += 1;

	Ok(())
}

/// Decrements the total number of file descriptors open system-wide.
fn decrement_total() {
	let mut guard = TOTAL_FD.lock();
	*guard.get_mut() -= 1;
}

/// Constraints to be respected when creating a new file descriptor.
#[derive(Debug)]
pub enum NewFDConstraint {
	/// No constraint
	None,
	/// The new file descriptor must have given fixed value
	Fixed(u32),
	/// The new file descriptor must have at least the given value
	Min(u32),
}

/// Structure representing a file descriptor.
#[derive(Clone)]
pub struct FileDescriptor {
	/// The FD's id.
	id: u32,
	/// The FD's flags.
	flags: i32,

	/// A pointer to the open file description associated with the file descriptor.
	open_file: SharedPtr<OpenFile>,
}

impl FileDescriptor {
	/// Creates a new file descriptor.
	pub fn new(id: u32, flags: i32, open_file: SharedPtr<OpenFile>) -> Self {
		Self {
			id,
			flags,

			open_file,
		}
	}

	/// Returns the file descriptor's ID.
	pub fn get_id(&self) -> u32 {
		self.id
	}

	/// Returns the file descriptor's flags.
	pub fn get_flags(&self) -> i32 {
		self.flags
	}

	/// Returns a pointer to the open file description.
	pub fn get_open_file(&self) -> SharedPtr<OpenFile> {
		self.open_file.clone()
	}
}
