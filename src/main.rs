use crate::{
    cli::Args,
    io::{IoBounds, IoMode},
};

use std::{
    fs::OpenOptions,
    io::{self as stdio},
    process,
};

fn main() {
    let stdout = stdio::stdout().lock();
    let stdin = stdio::stdin().lock();
    let mut stderr = stdio::stderr().lock();

    let io_bounds = IoBounds::new(stdout, stdin);
    let args = Args::new();
    let mut last_error: i32 = 0;

    if args.len() == 0 {
        IoMode::feed(IoMode::UnixPipe, io_bounds);
        return;
    }

    for path in args.iter() {
        let file = match OpenOptions::new()
            .read(true)
            .write(false)
            .append(false)
            .truncate(false)
            .create(false)
            .create_new(false)
            .open(path)
        {
            Ok(file) => file,
            Err(err) => {
                last_error = err.raw_os_error().unwrap_or(1);

                io::write(&mut stderr, format!("{err}\n").as_str());
                continue;
            }
        };

        IoMode::feed(IoMode::FileStream(file), memory::copy(&io_bounds));
    }

    process::exit(last_error);
}

mod cli {
    use std::{
        env,
        ops::{Deref, DerefMut},
    };

    #[derive(Debug, Clone)]
    pub struct Args {
        total: Vec<String>,
    }

    impl Args {
        pub fn new() -> Self {
            let args: Vec<String> = env::args()
                .enumerate()
                .filter(|(n, _)| *n != 0)
                .map(|(_, s)| s)
                .collect();

            Self { total: args }
        }
    }

    impl Deref for Args {
        type Target = Vec<String>;

        fn deref(&self) -> &Self::Target {
            &self.total
        }
    }

    impl DerefMut for Args {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.total
        }
    }
}

mod manipulate {
    pub fn reverse(src: &mut String) {
        let new_string: Vec<String> = src
            .split('\n')
            .map(|splitted| splitted.chars().rev().collect::<String>())
            .collect();

        src.clear();

        new_string
            .iter()
            .enumerate()
            .filter(|(index, _)| *index != new_string.len() - 1)
            .for_each(|(_, string)| {
                src.push_str(string);
                src.push('\n');
            });
    }
}

mod io {
    use super::{io, manipulate};

    use std::{
        fs::File,
        io::{Read, StdinLock, StdoutLock, Write},
    };

    #[derive(Debug)]
    pub struct IoBounds<'a>
    where
        'a: 'static,
    {
        pub stdout: StdoutLock<'a>,
        pub stdin: StdinLock<'a>,
    }

    impl<'a> IoBounds<'a>
    where
        'a: 'static,
    {
        pub fn new(stdout: StdoutLock<'a>, stdin: StdinLock<'a>) -> Self {
            Self { stdout, stdin }
        }
    }

    #[derive(Debug)]
    pub enum IoMode {
        UnixPipe,
        FileStream(File),
    }

    impl IoMode {
        pub fn feed<'a>(mode: Self, mut bounds: IoBounds<'a>) {
            let mut buf = String::new();

            match mode {
                Self::UnixPipe => {
                    io::read(&mut bounds.stdin, &mut buf);
                    manipulate::reverse(&mut buf);
                    buf.push('\n');
                    io::write(&mut bounds.stdout, buf.as_str());
                }

                Self::FileStream(mut file) => {
                    io::read(&mut file, &mut buf);
                    manipulate::reverse(&mut buf);
                    io::write(&mut bounds.stdout, buf.as_str());
                }
            }
        }
    }

    pub fn write<'a, T>(writeable: &'a mut T, s: &'a str)
    where
        T: 'a + Write,
    {
        writeable
            .write(s.as_bytes())
            .expect("Cannot write onto trait object <std::io::Write>");
    }

    pub fn read<'a, T>(readable: &'a mut T, buf: &mut String)
    where
        T: 'a + Read,
    {
        if buf.capacity() == 0 {
            buf.reserve(1024 * std::mem::size_of::<u8>());
        }

        readable
            .read_to_string(buf)
            .expect("Cannot read from trait object <std::io::Read>");
    }
}

mod memory {
    use std::mem;

    // @UNSAFE_FEATURES
    /// Im assuming that the alignment of `T` as `Src` and `T` as `Dst` is same as each other.
    /// which is `size_of::<<T as Src>>() == size_of::<<T as Dst>>()`
    unsafe fn copy_<'a, T>(src: &'a T) -> T {
        mem::transmute_copy::<T, T>(src)
    }

    // @WARNING
    /// Copy an instance of type `&T` into `T` using `std::mem::transmute_copy()`,
    /// with `Src` and `Dst` being T.
    pub fn copy<'a, T>(src: &'a T) -> T {
        unsafe { copy_(src) }
    }
}
