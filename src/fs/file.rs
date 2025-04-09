#[derive(Debug)]
#[repr(u64)]
pub enum XkcdFile {
    // Root,
    // Dir(u32),
    Num(u32),
    Title(u32),
    Image(u32),
    Alt(u32),
    Transcript(u32),
    ReleaseDate(u32),
}

pub enum XkcdDir {
    Root,
    Dir(u32),
}

impl XkcdDir {
    pub const fn inode(&self) -> u64 {
        match self {
            XkcdDir::Root => 1,
            XkcdDir::Dir(num) => ((*num as u64) << 32) | 2,
        }
    }

    pub fn name(&self) -> String {
        match self {
            XkcdDir::Root => ".".to_string(),
            XkcdDir::Dir(num) => format!("xkcd_{}", num),
        }
    }
}

impl XkcdFile {
    pub fn name(&self) -> String {
        match self {
            XkcdFile::Image(n) => format!("xkcd_{}.png", n),
            XkcdFile::Alt(n) => format!("xkcd_{}.alt", n),
            XkcdFile::Num(n) => format!("xkcd_{}.num", n),
            XkcdFile::Title(n) => format!("xkcd_{}.title", n),
            XkcdFile::Transcript(n) => format!("xkcd_{}.transcript", n),
            XkcdFile::ReleaseDate(n) => format!("xkcd_{}.release_date", n),
        }
    }

    pub(crate) fn inode(&self) -> u64 {
        match self {
            XkcdFile::Num(num) => ((*num as u64) << 32) | 3,
            XkcdFile::Title(num) => ((*num as u64) << 32) | 4,
            XkcdFile::Image(num) => ((*num as u64) << 32) | 5,
            XkcdFile::Alt(num) => ((*num as u64) << 32) | 6,
            XkcdFile::Transcript(num) => ((*num as u64) << 32) | 7,
            XkcdFile::ReleaseDate(num) => ((*num as u64) << 32) | 8,
        }
    }
}
