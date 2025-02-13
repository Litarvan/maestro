//! The procfs is a virtual filesystem which provides informations about processes.

pub mod mount;
pub mod root;

use crate::errno::Errno;
use crate::file::File;
use crate::file::FileContent;
use crate::file::Gid;
use crate::file::INode;
use crate::file::Mode;
use crate::file::Uid;
use crate::file::fs::Filesystem;
use crate::file::fs::FilesystemType;
use crate::file::fs::kernfs::KernFS;
use crate::file::path::Path;
use crate::util::IO;
use crate::util::boxed::Box;
use crate::util::container::string::String;
use crate::util::ptr::SharedPtr;
use root::ProcFSRoot;

/// Structure representing the procfs.
/// On the inside, the procfs works using a kernfs.
pub struct ProcFS {
	/// The kernfs.
	fs: KernFS,
}

impl ProcFS {
	/// Creates a new instance.
	/// `readonly` tells whether the filesystem is readonly.
	pub fn new(readonly: bool) -> Result<Self, Errno> {
		let mut fs = Self {
			fs: KernFS::new(String::from(b"procfs")?, readonly),
		};

		// Adding the root node
		let root_node = ProcFSRoot::new()?;
		fs.fs.set_root(Some(SharedPtr::new(root_node)?))?;

		Ok(fs)
	}
}

impl Filesystem for ProcFS {
	fn get_name(&self) -> &[u8] {
		self.fs.get_name()
	}

	fn is_readonly(&self) -> bool {
		self.fs.is_readonly()
	}

	fn must_cache(&self) -> bool {
		self.fs.must_cache()
	}

	fn get_root_inode(&self, _io: &mut dyn IO) -> Result<INode, Errno> {
		// TODO
		todo!();
	}

	fn get_inode(&mut self, io: &mut dyn IO, parent: Option<INode>, name: &String)
		-> Result<INode, Errno> {
		self.fs.get_inode(io, parent, name)
	}

	fn load_file(&mut self, io: &mut dyn IO, inode: INode, name: String)
		-> Result<File, Errno> {
		self.fs.load_file(io, inode, name)
	}

	fn add_file(&mut self, _io: &mut dyn IO, _parent_inode: INode, _name: String, _uid: Uid,
		_gid: Gid, _mode: Mode, _content: FileContent) -> Result<File, Errno> {
		Err(errno!(EPERM))
	}

	fn add_link(&mut self, _io: &mut dyn IO, _parent_inode: INode, _name: &String,
		_inode: INode) -> Result<(), Errno> {
		Err(errno!(EPERM))
	}

	fn update_inode(&mut self, _io: &mut dyn IO, _file: &File) -> Result<(), Errno> {
		Err(errno!(EPERM))
	}

	fn remove_file(&mut self, _io: &mut dyn IO, _parent_inode: INode, _name: &String)
		-> Result<(), Errno> {
		Err(errno!(EPERM))
	}

	fn read_node(&mut self, io: &mut dyn IO, inode: INode, off: u64, buf: &mut [u8])
		-> Result<u64, Errno> {
		self.fs.read_node(io, inode, off, buf)
	}

	fn write_node(&mut self, io: &mut dyn IO, inode: INode, off: u64, buf: &[u8])
		-> Result<(), Errno> {
		self.fs.write_node(io, inode, off, buf)
	}
}

/// Structure representing the procfs file system type.
pub struct ProcFsType {}

impl FilesystemType for ProcFsType {
	fn get_name(&self) -> &[u8] {
		b"procfs"
	}

	fn detect(&self, _io: &mut dyn IO) -> Result<bool, Errno> {
		Ok(false)
	}

	fn create_filesystem(&self, _io: &mut dyn IO) -> Result<Box<dyn Filesystem>, Errno> {
		Ok(Box::new(ProcFS::new(false)?)?)
	}

	fn load_filesystem(&self, _io: &mut dyn IO, _mountpath: Path, readonly: bool)
		-> Result<Box<dyn Filesystem>, Errno> {
		Ok(Box::new(ProcFS::new(readonly)?)?)
	}
}
