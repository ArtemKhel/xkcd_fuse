use std::path::Path;

use fuser::{MountOption, mount2};

use crate::storage::Storage;

pub mod file;
pub mod xkcd_fs;

pub fn fuse<St: Storage>(mount_point: &Path, storage: St) {
    let xkcd_fuse = xkcd_fs::XkcdFS::new(storage);
    let options = vec![MountOption::AutoUnmount, MountOption::AllowRoot];
    println!("Mounting xkcd at {}", mount_point.display());
    let _mount = mount2(xkcd_fuse, mount_point, &options);

    // let _mount = spawn_mount2(xkcd_fuse, &mount_point, &options);
    // let _ = io::stdin().read_line(&mut String::new()).unwrap();
}
