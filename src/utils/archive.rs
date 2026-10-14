use super::structs::{PGCliError, Result};
use clap::ValueEnum;
use log::{debug, info};
use std::fs::File;
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use tar::{Archive, Builder};
use tokio::task;
use xz2::read::XzDecoder;
use xz2::stream::{Check, MtStreamBuilder, Stream};
use xz2::write::XzEncoder;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Compression {
    Zstd,
    Xz,
}

impl Compression {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Zstd => "tar.zst",
            Self::Xz => "tar.xz",
        }
    }

    pub fn output_path(self, path: &Path) -> Result<PathBuf> {
        let name = path
            .file_name()
            .ok_or_else(|| PGCliError::Other("Archive output must include a file name".into()))?;
        let name_str = name.to_string_lossy();
        for codec in [Self::Zstd, Self::Xz] {
            if name_str.ends_with(&format!(".{}", codec.extension())) {
                return if codec == self {
                    Ok(path.to_path_buf())
                } else {
                    Err(PGCliError::Other(format!(
                        "Output suffix .{} conflicts with --compression {}; use .{}",
                        codec.extension(),
                        self.to_possible_value().unwrap().get_name(),
                        self.extension()
                    )))
                };
            }
        }
        let mut name = name.to_os_string();
        if name_str.ends_with(".tar") {
            name.push(match self {
                Self::Zstd => ".zst",
                Self::Xz => ".xz",
            });
        } else {
            name.push(format!(".{}", self.extension()));
        }
        Ok(path.with_file_name(name))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CompressionOptions {
    pub codec: Compression,
    pub level: u32,
    pub threads: u32,
}

impl CompressionOptions {
    pub fn new(codec: Compression, level: Option<u32>, threads: u32) -> Result<Self> {
        let level = level.unwrap_or(match codec {
            Compression::Zstd => 3,
            Compression::Xz => 9,
        });
        let valid = match codec {
            Compression::Zstd => (1..=22).contains(&level),
            Compression::Xz => level <= 9,
        };
        if !valid {
            return Err(PGCliError::Other(match codec {
                Compression::Zstd => "Zstd compression level must be between 1 and 22".into(),
                Compression::Xz => "XZ compression level must be between 0 and 9".into(),
            }));
        }
        if threads == 0 {
            return Err(PGCliError::Other(
                "Compression threads must be at least 1".into(),
            ));
        }
        Ok(Self {
            codec,
            level,
            threads,
        })
    }
}

/// Inspect the frame header so renamed archives can also be restored.
pub fn detect_compression(path: &Path) -> Result<Option<Compression>> {
    let mut header = Vec::with_capacity(6);
    File::open(path)?.take(6).read_to_end(&mut header)?;
    let codec = if header.starts_with(&[0x28, 0xb5, 0x2f, 0xfd]) {
        Some(Compression::Zstd)
    } else if header.starts_with(&[0xfd, 0x37, 0x7a, 0x58, 0x5a, 0x00]) {
        Some(Compression::Xz)
    } else {
        None
    };
    if codec.is_none()
        && path.file_name().is_some_and(|name| {
            let name = name.to_string_lossy();
            name.ends_with(".tar.zst") || name.ends_with(".tar.xz")
        })
    {
        return Err(PGCliError::Other(
            "Archive has an invalid or unsupported compression header".into(),
        ));
    }
    Ok(codec)
}

fn write_tar<W: Write>(input_dir: &Path, writer: W) -> Result<W> {
    let mut archive = Builder::new(writer);
    for entry in walkdir::WalkDir::new(input_dir) {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            debug!("Adding {:?}", path);
            archive.append_path_with_name(path, path.strip_prefix(input_dir)?)?;
        }
    }
    Ok(archive.into_inner()?)
}

pub async fn create_compressed_archive(
    input_dir: &Path,
    output_file: &Path,
    options: CompressionOptions,
) -> Result<()> {
    let input_dir = input_dir.to_path_buf();
    let output_file = output_file.to_path_buf();
    task::spawn_blocking(move || {
        if !input_dir.is_dir() {
            return Err(PGCliError::Other("Input directory does not exist".into()));
        }
        // Publish only after both the tar and compression streams finish successfully.
        let parent = output_file
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
        let writer = BufWriter::new(temporary.as_file_mut());
        let mut writer = match options.codec {
            Compression::Zstd => {
                let mut encoder = zstd::stream::write::Encoder::new(writer, options.level as i32)?;
                encoder.include_checksum(true)?;
                if options.threads > 1 {
                    encoder.multithread(options.threads)?;
                }
                write_tar(&input_dir, encoder)?.finish()?
            }
            Compression::Xz => {
                let stream = if options.threads == 1 {
                    Stream::new_easy_encoder(options.level, Check::Crc64)?
                } else {
                    let mut builder = MtStreamBuilder::new();
                    builder
                        .threads(options.threads)
                        .preset(options.level)
                        .check(Check::Crc64);
                    info!(
                        "Estimated XZ encoder memory: {} MiB",
                        builder.memusage() / (1024 * 1024)
                    );
                    builder.encoder()?
                };
                write_tar(&input_dir, XzEncoder::new_stream(writer, stream))?.finish()?
            }
        };
        writer.flush()?;
        drop(writer);
        temporary.as_file().sync_all()?;
        temporary
            .persist(&output_file)
            .map_err(|error| PGCliError::TokioIo(error.error))?;
        Ok(())
    })
    .await?
}

pub async fn extract_compressed_archive(input_file: &Path, output_dir: &Path) -> Result<()> {
    let input_file = input_file.to_path_buf();
    let output_dir = output_dir.to_path_buf();
    task::spawn_blocking(move || {
        let codec = detect_compression(&input_file)?
            .ok_or_else(|| PGCliError::Other("Unsupported archive compression format".into()))?;
        let file = BufReader::new(File::open(&input_file)?);
        let decoder: Box<dyn Read> = match codec {
            Compression::Zstd => Box::new(zstd::stream::read::Decoder::new(file)?),
            Compression::Xz => Box::new(XzDecoder::new_multi_decoder(file)),
        };
        let mut archive = Archive::new(decoder);
        archive.unpack(&output_dir)?;
        // Tar stops at its terminator. Read the compression footer too, checking
        // checksums and detecting truncated streams before allowing a DB restore.
        io::copy(&mut archive.into_inner(), &mut io::sink())?;
        Ok(())
    })
    .await?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_levels_and_threads() {
        assert_eq!(
            CompressionOptions::new(Compression::Zstd, None, 1)
                .unwrap()
                .level,
            3
        );
        assert_eq!(
            CompressionOptions::new(Compression::Xz, None, 1)
                .unwrap()
                .level,
            9
        );
        assert!(CompressionOptions::new(Compression::Xz, Some(0), 1).is_ok());
        for (codec, level) in [
            (Compression::Zstd, 0),
            (Compression::Zstd, 23),
            (Compression::Xz, 10),
        ] {
            assert!(CompressionOptions::new(codec, Some(level), 1).is_err());
        }
        assert!(CompressionOptions::new(Compression::Zstd, None, 0).is_err());
    }

    #[test]
    fn resolves_output_paths_and_rejects_conflicting_suffixes() {
        for codec in [Compression::Zstd, Compression::Xz] {
            let expected = PathBuf::from(format!("backup.{}", codec.extension()));
            for path in [Path::new("backup"), Path::new("backup.tar"), &expected] {
                assert_eq!(codec.output_path(path).unwrap(), expected);
            }
        }
        assert!(
            Compression::Zstd
                .output_path(Path::new("backup.tar.xz"))
                .is_err()
        );
        assert!(
            Compression::Xz
                .output_path(Path::new("backup.tar.zst"))
                .is_err()
        );
    }

    #[tokio::test]
    async fn round_trip_single_and_parallel_archives_and_check_integrity() {
        let workspace = tempfile::tempdir().unwrap();
        let input = workspace.path().join("input");
        std::fs::create_dir_all(input.join("nested")).unwrap();
        let payload: Vec<u8> = (0..8 * 1024 * 1024).map(|i| (i % 251) as u8).collect();
        std::fs::write(input.join("nested/data.bsql"), &payload).unwrap();
        std::fs::write(input.join("empty.sql"), []).unwrap();
        for codec in [Compression::Zstd, Compression::Xz] {
            for threads in [1, 2] {
                let archive = workspace
                    .path()
                    .join(format!("backup-{codec:?}-{threads}.{}", codec.extension()));
                let options = CompressionOptions::new(codec, Some(1), threads).unwrap();
                create_compressed_archive(&input, &archive, options)
                    .await
                    .unwrap();
                assert_eq!(detect_compression(&archive).unwrap(), Some(codec));
                let output = tempfile::tempdir().unwrap();
                extract_compressed_archive(&archive, output.path())
                    .await
                    .unwrap();
                assert_eq!(
                    std::fs::read(output.path().join("nested/data.bsql")).unwrap(),
                    payload
                );
                assert_eq!(
                    std::fs::metadata(output.path().join("empty.sql"))
                        .unwrap()
                        .len(),
                    0
                );

                let bytes = std::fs::read(&archive).unwrap();
                let renamed = workspace.path().join("renamed.backup");
                std::fs::write(&renamed, &bytes).unwrap();
                assert_eq!(detect_compression(&renamed).unwrap(), Some(codec));
                extract_compressed_archive(&renamed, output.path())
                    .await
                    .unwrap();

                std::fs::write(&archive, &bytes[..bytes.len() - 1]).unwrap();
                assert!(
                    extract_compressed_archive(&archive, output.path())
                        .await
                        .is_err()
                );
                let mut corrupt = bytes;
                let offset = corrupt.len() - 5;
                corrupt[offset] ^= 0xff;
                std::fs::write(&archive, corrupt).unwrap();
                assert!(
                    extract_compressed_archive(&archive, output.path())
                        .await
                        .is_err()
                );
            }
        }
    }

    #[tokio::test]
    async fn failed_archive_does_not_replace_existing_output() {
        let workspace = tempfile::tempdir().unwrap();
        let output = workspace.path().join("backup.tar.zst");
        std::fs::write(&output, b"existing backup").unwrap();
        let options = CompressionOptions::new(Compression::Zstd, None, 1).unwrap();
        assert!(
            create_compressed_archive(&workspace.path().join("missing"), &output, options)
                .await
                .is_err()
        );
        assert_eq!(std::fs::read(&output).unwrap(), b"existing backup");
    }

    #[test]
    fn rejects_invalid_archive_headers_but_accepts_raw_dumps() {
        let workspace = tempfile::tempdir().unwrap();
        for name in ["backup.tar.zst", "backup.tar.xz"] {
            let path = workspace.path().join(name);
            std::fs::write(&path, b"invalid").unwrap();
            assert!(detect_compression(&path).is_err());
        }
        let raw = workspace.path().join("backup.bsql");
        std::fs::write(&raw, b"PGDMP").unwrap();
        assert_eq!(detect_compression(&raw).unwrap(), None);
    }
}
