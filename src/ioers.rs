use super::*;

use std::{
    fmt::Debug,
    fs::File,
    io::{BufReader, BufWriter},
};

pub trait InputOutput: Input + Output {}
pub trait Input: Debug {
    fn reader<'a>(&'a mut self) -> io::Result<&'a mut dyn io::Read>;
}
pub trait Output: Debug {
    fn writer<'a>(&'a mut self) -> io::Result<&'a mut dyn io::Write>;
    fn extension<'a>(&'a self) -> Option<&'a OsStr> {
        None
    }

    /// Take effects before [`Output::get_writer`].
    #[allow(unused_variables)]
    fn set_file_name(&mut self, file_name: &OsStr) -> io::Result<()> {
        Err(io::Error::new(io::ErrorKind::Unsupported, "unsupported"))
    }

    /// Remove only if the DST file is empty. `bool` here indicates whether the file is empty.
    fn remove_dst(&mut self) -> io::Result<bool> {
        Err(io::Error::new(io::ErrorKind::Unsupported, "unsupported"))
    }
    /// **Anyway** remove the DST file.
    fn remove_dst_anyway(&mut self) -> io::Result<()> {
        Err(io::Error::new(io::ErrorKind::Unsupported, "unsupported"))
    }
}

/// Just a simple wrapper.
#[derive(Debug)]
pub struct ClarifiedIo {
    i: Box<dyn Input>,
    o: Box<dyn Output>,
}
impl ClarifiedIo {
    pub fn new<I, O>(i: I, o: O) -> Self
    where
        I: 'static + Input,
        O: 'static + Output,
    {
        Self {
            i: Box::new(i),
            o: Box::new(o),
        }
    }

    pub fn with_input<I: 'static + Input>(&mut self, i: I) {
        self.i = Box::new(i);
    }
    pub fn with_output<O: 'static + Output>(&mut self, o: O) {
        self.o = Box::new(o);
    }
}
impl InputOutput for ClarifiedIo {}
impl Input for ClarifiedIo {
    fn reader<'a>(&'a mut self) -> io::Result<&'a mut dyn io::Read> {
        self.i.reader()
    }
}
impl Output for ClarifiedIo {
    fn writer<'a>(&'a mut self) -> io::Result<&'a mut dyn io::Write> {
        self.o.writer()
    }
    fn extension<'a>(&'a self) -> Option<&'a OsStr> {
        self.o.extension()
    }
    fn set_file_name(&mut self, file_name: &OsStr) -> io::Result<()> {
        self.o.set_file_name(file_name)
    }
    fn remove_dst(&mut self) -> io::Result<bool> {
        self.o.remove_dst()
    }
    fn remove_dst_anyway(&mut self) -> io::Result<()> {
        self.o.remove_dst_anyway()
    }
}

#[derive(Debug)]
pub struct ReadFile {
    src: PathBuf,
    reader: Option<BufReader<File>>,
}
impl ReadFile {
    pub fn new<P: AsRef<Path>>(src: P) -> Self {
        Self {
            src: src.as_ref().to_owned(),
            reader: None,
        }
    }
}
impl Input for ReadFile {
    fn reader<'a>(&'a mut self) -> io::Result<&'a mut dyn io::Read> {
        if self.reader.is_none() {
            self.reader = Some(BufReader::new(File::open(&self.src)?));
        }
        Ok(self.reader.as_mut().unwrap())
    }
}

#[derive(Debug)]
pub struct WriteFile {
    dst: PathBuf,
    writer: Option<BufWriter<File>>,
}
impl WriteFile {
    pub fn new<P: AsRef<Path>>(dst: P) -> Self {
        Self {
            dst: dst.as_ref().to_owned(),
            writer: None,
        }
    }
}
impl Output for WriteFile {
    fn writer<'a>(&'a mut self) -> io::Result<&'a mut dyn io::Write> {
        if self.writer.is_none() {
            self.writer = Some(BufWriter::new(File::create(&self.dst)?));
        }
        Ok(self.writer.as_mut().unwrap())
    }
    fn extension<'a>(&'a self) -> Option<&'a OsStr> {
        Some(match self.dst.extension() {
            Some(ext) => ext,
            None => OsStr::new(""),
        })
    }

    fn set_file_name(&mut self, file_name: &OsStr) -> io::Result<()> {
        Ok(self.dst.set_file_name(file_name))
    }

    fn remove_dst(&mut self) -> io::Result<bool> {
        match self.dst.metadata()?.len() == 0 {
            true => self.remove_dst_anyway().and(Ok(true)),
            false => Ok(false),
        }
    }
    fn remove_dst_anyway(&mut self) -> io::Result<()> {
        drop(self.writer.take());
        fs::remove_file(&self.dst)
    }
}

#[derive(Debug)]
pub struct ReadStdin(io::Stdin);
impl ReadStdin {
    pub fn new() -> Self {
        Self(io::stdin())
    }
}
impl Input for ReadStdin {
    fn reader<'a>(&'a mut self) -> io::Result<&'a mut dyn io::Read> {
        Ok(&mut self.0)
    }
}

#[derive(Debug)]
pub struct WriteStdout(io::Stdout);
impl WriteStdout {
    pub fn new() -> Self {
        Self(io::stdout())
    }
}
impl Output for WriteStdout {
    fn writer<'a>(&'a mut self) -> io::Result<&'a mut dyn io::Write> {
        Ok(&mut self.0)
    }
}
