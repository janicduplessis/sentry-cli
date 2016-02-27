use std::io;
use std::fs;
use std::env;
use std::io::Seek;
use std::os::unix::io::{AsRawFd, FromRawFd};
use uuid::Uuid;
use chan;
use chan_signal::{notify, Signal};
use libc;

pub struct TempFile {
    f: fs::File,
}

impl TempFile {
    pub fn new() -> io::Result<TempFile> {
        let mut path = env::temp_dir();
        path.push(Uuid::new_v4().to_hyphenated_string());
        let f = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&path).unwrap();
        let _ = fs::remove_file(&path);
        Ok(TempFile {
            f: f
        })
    }

    pub fn dup<T: FromRawFd>(&self) -> T {
        let raw_fd = self.f.as_raw_fd();
        unsafe {
            FromRawFd::from_raw_fd(libc::dup(raw_fd))
        }
    }

    pub fn open(&self) -> fs::File {
        let mut f : fs::File = self.dup();
        let _ = f.seek(io::SeekFrom::Start(0));
        f
    }
}

pub fn run_or_interrupt<F>(f: F) -> Option<Signal>
    where F: FnOnce() -> (), F: Send + 'static
{
    let run = |_sdone: chan::Sender<()>| f();
    let signal = notify(&[Signal::INT, Signal::TERM]);
    let (sdone, rdone) = chan::sync(0);
    ::std::thread::spawn(move || run(sdone));

    let mut rv = None;

    chan_select! {
        signal.recv() -> signal => { rv = signal; },
        rdone.recv() => {}
    }

    rv
}