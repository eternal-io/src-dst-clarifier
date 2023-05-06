use std::{
    ffi::{OsStr, OsString},
    fs, io,
    path::{Path, PathBuf},
};

use thiserror::Error;

use kalavor::Katetime;

/// Use single hyphen (`-`) as path to indicate IO from Stdio.
///
/// # Notes
///
/// - Auto time-based unique naming (`auto_tnamed_dst_`) only takes effect when DST is not provided.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SrcDstConfig {
    pub allow_from_stdin: bool,
    pub allow_to_stdout: bool,

    pub auto_tnamed_dst_file: bool,
    pub auto_tnamed_dst_dir: bool,

    pub default_extension: OsString,

    /// Disallowed by default. There may be a potential to `open` and `create` the same file at the same time.
    pub allow_inplace: bool,
}

impl SrcDstConfig {
    pub fn new<S: AsRef<OsStr>>(default_extension: S) -> Self {
        Self {
            allow_from_stdin: true,
            allow_to_stdout: true,
            auto_tnamed_dst_file: true,
            auto_tnamed_dst_dir: true,
            default_extension: default_extension.as_ref().to_owned(),
            allow_inplace: false,
        }
    }

    pub fn new_with_allow_inplace<S: AsRef<OsStr>>(default_extension: S) -> Self {
        Self {
            allow_from_stdin: true,
            allow_to_stdout: true,
            auto_tnamed_dst_file: true,
            auto_tnamed_dst_dir: true,
            default_extension: default_extension.as_ref().to_owned(),
            allow_inplace: true,
        }
    }

    /// # Possible Combinations
    ///
    /// ``` plaintext
    /// SRC => DST:   Stdout,6   File   Dir     NotProvided
    /// Stdin,6          1+2      1      1         1+3,5
    /// File               2      ✓      ✓           3
    /// Dir                ×      ×      ✓           4
    /// ```
    ///
    /// 1. `allow_from_stdin`.
    /// 2. `allow_to_stdout`.
    /// 3. `auto_tnamed_dst_file`.
    /// 4. `auto_tnamed_dst_dir`.
    ///     Note: A directory with specified name will not be created automatically
    ///     (an error will be returned if it does not exist).
    /// 5. Note that [`std::env::current_dir`] will be used as output directory.
    /// 6. *Stdio will be always treated as a file.*
    pub fn parse<P: AsRef<Path>>(
        &self,
        src: P,
        dst: Option<P>,
    ) -> io::Result<Result<SrcDstPairs, SrcDstError>> {
        enum InnerSource {
            Stdin,
            File(PathBuf),
            Dir(PathBuf),
        }

        enum InnerDrain {
            Stdout,
            File(PathBuf),
            Dir(PathBuf),
            NotExist(PathBuf),
            NotProvided,
        }

        let src = src.as_ref();
        let src = if src.as_os_str() == "-" {
            InnerSource::Stdin
        } else if !src.exists() {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                format!("SRC '{}' does not exist", src.to_string_lossy()),
            ));
        } else {
            let src = fs::canonicalize(src)?;
            if src.is_file() {
                InnerSource::File(src)
            } else {
                InnerSource::Dir(src)
            }
        };

        let dst = dst.as_ref();
        let mut dst = match dst {
            None => InnerDrain::NotProvided,
            Some(dst) => {
                let dst = dst.as_ref();
                if dst.as_os_str() == "-" {
                    InnerDrain::Stdout
                } else if !dst.exists() {
                    InnerDrain::NotExist(dst.to_owned())
                } else {
                    let dst = fs::canonicalize(dst)?;
                    if dst.is_file() {
                        InnerDrain::File(dst)
                    } else {
                        InnerDrain::Dir(dst)
                    }
                }
            }
        };

        if matches!(src, InnerSource::Stdin) && !self.allow_from_stdin {
            return Ok(Err(SrcDstError::DisallowFromStdin)); // 1
        }
        if matches!(dst, InnerDrain::Stdout) && !self.allow_to_stdout {
            return Ok(Err(SrcDstError::DisallowToStdout)); // 2
        }
        if matches!(dst, InnerDrain::NotProvided) {
            if matches!(src, InnerSource::Dir(_)) && !self.auto_tnamed_dst_dir {
                return Ok(Err(SrcDstError::ForbidAutoTnamedDstDir)); // 4
            } else if !self.auto_tnamed_dst_file {
                return Ok(Err(SrcDstError::ForbidAutoTnamedDstFile)); // 3
            }
        }
        if let InnerDrain::Dir(parent) = &dst {
            if let InnerSource::File(src) = &src {
                if fs::canonicalize(parent)? == fs::canonicalize(src)?.parent().unwrap() {
                    dst = InnerDrain::NotProvided; // 当 DST-Dir 与 SRC-File所在目录 相同时，切换至 tname
                }
            } else if !self.allow_inplace {
                if let InnerSource::Dir(src) = &src {
                    if fs::canonicalize(parent)? == fs::canonicalize(src)? {
                        return Ok(Err(SrcDstError::Inplaced));
                    }
                }
            }
        }

        let mut tnamed = false;
        let (src, dst): (Source, Drain) = match src {
            InnerSource::Stdin | InnerSource::File(_) => {
                fn dst_parent_src_name(src: &InnerSource, dst: &InnerDrain) -> io::Result<PathBuf> {
                    let mut parent = match dst {
                        InnerDrain::Dir(parent) => parent.to_owned(),
                        InnerDrain::NotProvided => fs::canonicalize(std::env::current_dir()?)?,
                        _ => unreachable!(),
                    };
                    parent.push(match src {
                        InnerSource::Stdin => OsString::from("stdin"),
                        InnerSource::File(src) => src.file_name().unwrap().into(), // 在调用这个函数时，SRC 已经规范化了
                        InnerSource::Dir(_) => unreachable!(),
                    });

                    Ok(parent)
                }

                (
                    match &src {
                        InnerSource::Stdin => Source::Stdin,
                        InnerSource::File(src) => Source::File(src.clone()),
                        InnerSource::Dir(_) => unreachable!(),
                    },
                    match dst {
                        InnerDrain::Stdout => Drain::Stdout,
                        InnerDrain::File(dst) => Drain::Single(dst),
                        InnerDrain::Dir(_) => Drain::Single(dst_parent_src_name(&src, &dst)?),
                        InnerDrain::NotExist(dst) => Drain::Single(dst),
                        InnerDrain::NotProvided => {
                            // input.png => input-A01123-0456-0789.png
                            // input.jpg => input.jpg-A01123-0456-0789.png

                            let mut dst = dst_parent_src_name(&src, &dst)?;

                            dst.extension()
                                .and_then(|ext| Some(ext == self.default_extension))
                                .unwrap_or(false)
                                .then(|| dst.set_extension("")); // 如果后缀不错，那么就去掉
                            dst.set_file_name(format!(
                                "{}-{}{}",
                                dst.as_os_str().to_string_lossy(),
                                Katetime::now_datetime(),
                                match self.default_extension.is_empty() {
                                    true => String::with_capacity(0),
                                    false =>
                                        format!(".{}", self.default_extension.to_string_lossy()),
                                }
                            ));

                            Drain::Single(dst)
                        }
                    },
                )
            }

            InnerSource::Dir(src) => {
                fn shallow_walk<P: AsRef<Path>>(src: P) -> io::Result<Vec<PathBuf>> {
                    let mut files = fs::read_dir(src)?
                        .filter_map(Result::ok)
                        .filter_map(|p| {
                            p.metadata()
                                .ok()
                                .and_then(|m| m.is_file().then(|| p.path()))
                        })
                        .collect::<Vec<_>>();
                    files.sort_unstable_by(|a, b| b.cmp(a));
                    Ok(files)
                }

                match dst {
                    InnerDrain::Stdout => return Ok(Err(SrcDstError::ManyToOne)),
                    InnerDrain::File(_) => return Ok(Err(SrcDstError::ManyToOne)),
                    InnerDrain::Dir(dst) => (Source::Files(shallow_walk(src)?), Drain::Single(dst)),
                    InnerDrain::NotExist(_) => return Ok(Err(SrcDstError::DstDirNotExist)),
                    InnerDrain::NotProvided => {
                        // ./inputs => ./inputs-A01123-0456-0789
                        let mut dst = src
                            .parent()
                            .ok_or_else(|| {
                                io::Error::new(
                                    io::ErrorKind::PermissionDenied,
                                    format!("parent directory of {src:?} are unavailable"),
                                )
                            })?
                            .to_owned();
                        dst.push(format!(
                            "{}-{}",
                            src.file_name().unwrap().to_string_lossy(),
                            Katetime::now_datetime()
                        ));

                        tnamed = true;
                        (Source::Files(shallow_walk(src)?), Drain::Single(dst))
                    }
                }
            }
        };

        Ok(Ok(SrcDstPairs {
            src,
            dst,
            tnamed_dir: tnamed,
            finished: false,
        }))
    }
}

#[non_exhaustive]
#[derive(Error, Debug, Clone, Copy, PartialEq, Eq)]
pub enum SrcDstError {
    #[error("disallow read from stdin")]
    DisallowFromStdin = 1,
    #[error("disallow write to stdout")]
    DisallowToStdout,
    #[error("forbid automatic time-based named DST file")]
    ForbidAutoTnamedDstFile,
    #[error("forbid automatic time-based named DST directory")]
    ForbidAutoTnamedDstDir,

    #[error("there may be a potential to `open` and `create` the same file at the same time")]
    Inplaced,

    #[error("unable to write multiple files to one file")]
    ManyToOne,
    #[error("specified DST directory does not exist")]
    DstDirNotExist,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Src {
    File(PathBuf),
    Stdin,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Dst {
    File(PathBuf),
    Stdout,
}

#[derive(Debug)]
pub struct SrcDstPairs {
    src: Source,
    dst: Drain,

    tnamed_dir: bool,
    finished: bool,
}

impl SrcDstPairs {
    /// **Before consuming the path pair, call this method to create time-based named directory!**
    pub fn create_tnamed_dir(&self) -> io::Result<()> {
        if let Drain::Single(dir) = &self.dst {
            if self.tnamed_dir {
                fs::create_dir(dir)?;
            }
        }
        Ok(())
    }

    pub fn is_batch(&self) -> bool {
        matches!(self.src, Source::Files(_))
    }
}

impl Iterator for SrcDstPairs {
    type Item = (Src, Dst);

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }

        match &self.dst {
            Drain::Stdout => match &self.src {
                Source::Stdin => {
                    self.finished = true;
                    Some((Src::Stdin, Dst::Stdout))
                }
                Source::File(src) => {
                    self.finished = true;
                    Some((Src::File(src.to_owned()), Dst::Stdout))
                }
                Source::Files(_) => unreachable!(),
            },

            Drain::Single(dst) => match &mut self.src {
                Source::Stdin => {
                    self.finished = true;
                    Some((Src::Stdin, Dst::File(dst.to_owned())))
                }
                Source::File(src) => {
                    self.finished = true;
                    Some((Src::File(src.to_owned()), Dst::File(dst.to_owned())))
                }
                Source::Files(srcs) => match srcs.pop() {
                    None => None,
                    Some(src) => {
                        let dst = dst.join(src.file_name().unwrap());
                        Some((Src::File(src), Dst::File(dst)))
                    }
                },
            },
        }
    }
}

#[derive(Debug)]
enum Source {
    Stdin,
    File(PathBuf),
    /// 注意文件列表应该是倒过来排序的！这样就能把它们一个个 pop 出来了。
    Files(Vec<PathBuf>),
}

#[derive(Debug)]
enum Drain {
    Stdout,
    /// 注意这玩意必须手动拼接！（如果 SRC 是 [`Source::Files`] 的话）也就是文件名相同，但父目录不同。
    Single(PathBuf),
}

/// 我该怎么做测试？只是简单跑一下`cargo test -- --nocapture`吗？
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        match SrcDstConfig::new("png").parse(".", None) {
            Err(e) => println!("{e}"),
            Ok(p) => match p {
                Err(e) => println!("{e}"),
                Ok(p) => {
                    let p = p.collect::<Vec<_>>();
                    println!("{p:#?}")
                }
            },
        };
    }
}
