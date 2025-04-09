use std::{
    cmp::min,
    collections::HashMap,
    ffi::{OsStr, OsString},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use fuser::{FileAttr, FileType, Filesystem, KernelConfig, ReplyAttr, ReplyData, ReplyDirectory, ReplyEntry, Request};
use libc::{EISDIR, ENOENT, c_int};
use log::{info, warn};

use crate::{
    fs::file::{XkcdDir, XkcdFile},
    storage::Storage,
    xkcd::Xkcd,
};

#[derive(Debug)]
pub struct XkcdFS<S: Storage> {
    inodes: HashMap<u64, INode>,
    ttl: Duration,
    storage: S,
}

#[derive(Debug)]
pub struct INode {
    attrs: FileAttr,
    kind: INodeKind,
}

impl INode {
    fn dir(ino: u64, parent: Option<u64>, time: SystemTime) -> Self {
        INode {
            attrs: {
                FileAttr {
                    ino,
                    size: 0,
                    blocks: 0,
                    atime: time,
                    mtime: time,
                    ctime: time,
                    crtime: time,
                    kind: FileType::Directory,
                    perm: 0o555,
                    nlink: 2,
                    uid: 1000,
                    gid: 10,
                    rdev: 0,
                    flags: 0,
                    blksize: 512,
                }
            },
            kind: INodeKind::Directory(Directory {
                children: HashMap::new(),
                parent,
            }),
        }
    }
}

#[derive(Debug)]
enum INodeKind {
    Directory(Directory),
    File(XkcdFile),
}

#[derive(Debug)]
struct Directory {
    children: HashMap<OsString, u64>,
    parent: Option<u64>,
}

impl<St: Storage> XkcdFS<St> {
    const ROOT_INO: u64 = XkcdDir::Root.inode();

    pub fn new(xkcd_storage: St) -> Self {
        let mut inodes = HashMap::new();
        inodes.insert(Self::ROOT_INO, INode::dir(Self::ROOT_INO, None, UNIX_EPOCH));

        Self {
            inodes,
            ttl: Duration::from_secs(60),
            storage: xkcd_storage,
        }
    }

    fn init_file_attr(ino: u64, time: SystemTime, size: u64) -> FileAttr {
        FileAttr {
            ino,
            size,
            blocks: 0,
            atime: time,
            mtime: time,
            ctime: time,
            crtime: time,
            kind: FileType::RegularFile,
            perm: 0o444,
            nlink: 2,
            uid: 1000,
            gid: 10,
            rdev: 0,
            flags: 0,
            blksize: 512,
        }
    }

    fn init_meta_file(storage: &St, meta: &Xkcd, file: &XkcdFile) -> (FileAttr, OsString) {
        let file_ino = file.inode();
        let sys_time = meta.release_date_as_timestamp();

        let size = match file {
            XkcdFile::Num(n) => n.to_string().len() as u64,
            XkcdFile::Title(_) => meta.title.len() as u64,
            XkcdFile::Image(_) => storage.get_image_size(meta.num).unwrap_or(0),
            XkcdFile::Alt(_) => meta.alt.len() as u64,
            XkcdFile::Transcript(_) => meta.transcript.len() as u64,
            XkcdFile::ReleaseDate(_) => meta.release_date.to_string().len() as u64,
        };

        let file_attr = XkcdFS::<St>::init_file_attr(file_ino, sys_time, size);
        let name = OsString::from(file.name().to_string());
        (file_attr, name)
    }

    fn init_meta_files(storage: &St, meta: &Xkcd) -> Vec<(XkcdFile, FileAttr, OsString)> {
        let mut meta_files = vec![];
        let files = [
            XkcdFile::Image(meta.num),
            XkcdFile::Num(meta.num),
            XkcdFile::Title(meta.num),
            XkcdFile::Alt(meta.num),
            XkcdFile::Transcript(meta.num),
            XkcdFile::ReleaseDate(meta.num),
        ];
        for file in files {
            let (file_attr, name) = Self::init_meta_file(storage, meta, &file);
            meta_files.push((file, file_attr, name));
        }
        meta_files
    }

    fn init_dir(storage: &St, inodes: &mut HashMap<u64, INode>, meta: &Xkcd) -> (OsString, u64) {
        let dir = XkcdDir::Dir(meta.num);
        let ino = dir.inode();
        let mut dir_inode = INode::dir(ino, Some(Self::ROOT_INO), meta.release_date_as_timestamp());

        Self::init_dir_contents(storage, inodes, meta, &mut dir_inode);

        inodes.insert(ino, dir_inode);
        (dir.name().into(), ino)
    }

    fn init_dir_contents(storage: &St, inodes: &mut HashMap<u64, INode>, meta: &Xkcd, meta_inode: &mut INode) {
        let INodeKind::Directory(dir) = &mut meta_inode.kind else {
            panic!("Expected dir INode")
        };

        let meta_files = Self::init_meta_files(storage, meta);

        for (file, file_attr, name) in meta_files.into_iter() {
            dir.children.insert(name, file_attr.ino);
            inodes.insert(file_attr.ino, INode {
                attrs: file_attr,
                kind: INodeKind::File(file),
            });
        }
    }
}

impl<S: Storage> Filesystem for XkcdFS<S> {
    fn init(&mut self, _req: &Request<'_>, _config: &mut KernelConfig) -> Result<(), c_int> {
        let mut new_root_children = vec![];
        let _ = &self
            .storage
            .get_stored_ids()
            .into_iter()
            .filter_map(|id| self.storage.get_meta(id))
            .for_each(|meta| new_root_children.push(Self::init_dir(&self.storage, &mut self.inodes, &meta)));

        if let Some(root) = self.inodes.get_mut(&Self::ROOT_INO)
            && let INodeKind::Directory(root_dir) = &mut root.kind
        {
            for (name, ino) in new_root_children {
                root_dir.children.insert(name, ino);
            }
            Ok(())
        } else {
            Err(255)
        }
    }

    fn lookup(&mut self, _req: &Request<'_>, parent: u64, name: &OsStr, reply: ReplyEntry) {
        info!("lookup: parent = {}, name = {:?}", parent, name);
        let Some(parent_inode) = self.inodes.get(&parent) else {
            reply.error(ENOENT);
            return;
        };

        match &parent_inode.kind {
            INodeKind::Directory(dir) => {
                if let Some(child_ino) = dir.children.get(name)
                    && let Some(child_inode) = self.inodes.get(child_ino)
                {
                    reply.entry(&self.ttl, &child_inode.attrs, 0);
                } else {
                    reply.error(ENOENT);
                };
            }
            INodeKind::File(_) => {
                if let Some(dir_inode) = self.inodes.get(&parent)
                    && let INodeKind::Directory(dir) = &dir_inode.kind
                    && let Some(child_ino) = dir.children.get(name)
                    && let Some(child_inode) = self.inodes.get(child_ino)
                {
                    reply.entry(&self.ttl, &child_inode.attrs, 0);
                } else {
                    reply.error(ENOENT);
                }
            }
        };
    }

    fn getattr(&mut self, _req: &Request<'_>, ino: u64, _fh: Option<u64>, reply: ReplyAttr) {
        info!("getattr: ino = {}", ino);
        let Some(attr) = self.inodes.get(&ino).map(|inode| inode.attrs) else {
            reply.error(ENOENT);
            return;
        };
        reply.attr(&self.ttl, &attr);
    }

    fn read(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        _fh: u64,
        offset: i64,
        size: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: ReplyData,
    ) {
        let offset = offset as u64;
        info!("read: ino = {}, offset = {}, size = {}", ino, offset, size);

        let Some(inode) = self.inodes.get(&ino) else {
            warn!("ino not found");
            reply.error(ENOENT);
            return;
        };

        match &inode.kind {
            INodeKind::Directory(_) => {
                reply.error(EISDIR);
            }
            INodeKind::File(file) => {
                let read_size = min(size as u64, inode.attrs.size.saturating_sub(offset));
                let slice = (offset as usize)..(offset + read_size) as usize;
                match *file {
                    XkcdFile::Image(num) => {
                        let Some(image) = self.storage.get_image(num) else {
                            reply.error(ENOENT);
                            return;
                        };
                        reply.data(&image[slice]);
                    }
                    XkcdFile::Num(num)
                    | XkcdFile::Title(num)
                    | XkcdFile::Alt(num)
                    | XkcdFile::Transcript(num)
                    | XkcdFile::ReleaseDate(num) => {
                        if let Some(meta) = self.storage.get_meta(num) {
                            let data = match *file {
                                XkcdFile::Num(_) => meta.num.to_string(),
                                XkcdFile::Title(_) => meta.title.clone(),
                                XkcdFile::Alt(_) => meta.alt.clone(),
                                XkcdFile::Transcript(_) => meta.transcript.clone(),
                                XkcdFile::ReleaseDate(_) => meta.release_date.to_string(),
                                _ => unreachable!(),
                            };
                            reply.data(data[slice].as_bytes());
                        } else {
                            reply.error(ENOENT);
                        }
                    }
                }
            }
        }
    }

    fn readdir(&mut self, _req: &Request<'_>, ino: u64, _fh: u64, offset: i64, mut reply: ReplyDirectory) {
        info!("readdir: ino = {}, offset = {}", ino, offset);

        let Some(inode) = self.inodes.get(&ino) else {
            reply.error(ENOENT);
            return;
        };

        match &inode.kind {
            INodeKind::Directory(dir) => {
                if offset <= 0
                    && reply.add(
                        inode.attrs.ino,
                        (inode.attrs.ino + 1) as i64,
                        FileType::Directory,
                        OsStr::new("."),
                    )
                {
                    reply.ok();
                    return;
                };

                if offset <= 1
                    && let Some(parent) = dir.parent
                    && reply.add(parent, 1, FileType::Directory, OsStr::new(".."))
                {
                    reply.ok();
                    return;
                };

                for (i, (name, ino)) in dir.children.iter().enumerate().skip(offset as usize) {
                    let child = self.inodes.get(ino).unwrap();
                    let kind = match &child.kind {
                        INodeKind::Directory(_) => FileType::Directory,
                        INodeKind::File(_) => FileType::RegularFile,
                    };
                    if reply.add(*ino, (i + 1) as i64, kind, name) {
                        break;
                    }
                }
                reply.ok();
            }
            INodeKind::File(_) => reply.error(ENOENT),
        }
    }
}
