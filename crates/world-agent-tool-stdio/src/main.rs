use serde_json::Value;
use std::env;
use std::error::Error;
use std::fmt;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::path::PathBuf;
use world_agent_tool_host::{ReadOnlyJsonToolHost, ReadOnlyJsonToolHostProtocolError};
use world_agent_tools::{FirstDivergenceTool, ReadOnlyJsonToolRegistry};
use world_investigation_local::LocalArchiveComparisonExecutor;

const USAGE: &str = "Usage: world-agent-tool-stdio <left.world> <right.world>\n\nReads one world-machine-readonly-tools JSON request per stdin line and writes one response per line. The World archive pair is fixed at process startup.";
const MAX_TOOL_REQUEST_BYTES: usize = 64 * 1024 * 1024;

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.as_slice() == ["--help"] || args.as_slice() == ["-h"] {
        println!("{USAGE}");
        return Ok(());
    }
    let [left, right] = args.as_slice() else {
        return Err(Box::new(StdioCliError(format!(
            "invalid arguments\n\n{USAGE}"
        ))));
    };

    let executor = LocalArchiveComparisonExecutor::from_archive_paths(
        &PathBuf::from(left),
        &PathBuf::from(right),
    )?;
    let mut registry = ReadOnlyJsonToolRegistry::new();
    registry.register(FirstDivergenceTool::new(executor))?;
    let mut host = ReadOnlyJsonToolHost::new(registry);

    let stdin = io::stdin();
    let stdout = io::stdout();
    serve_json_lines(
        &mut host,
        BufReader::new(stdin.lock()),
        BufWriter::new(stdout.lock()),
    )?;
    Ok(())
}

fn serve_json_lines<E, R, W>(
    host: &mut ReadOnlyJsonToolHost<E>,
    reader: R,
    writer: W,
) -> Result<(), StdioAdapterError>
where
    E: fmt::Display,
    R: BufRead,
    W: Write,
{
    serve_json_lines_with_limit(host, reader, writer, MAX_TOOL_REQUEST_BYTES)
}

fn serve_json_lines_with_limit<E, R, W>(
    host: &mut ReadOnlyJsonToolHost<E>,
    mut reader: R,
    mut writer: W,
    max_bytes: usize,
) -> Result<(), StdioAdapterError>
where
    E: fmt::Display,
    R: BufRead,
    W: Write,
{
    let mut line_number = 0;
    loop {
        let next_line = line_number + 1;
        let Some(line) = read_bounded_json_line(&mut reader, max_bytes, next_line)? else {
            break;
        };
        line_number = next_line;
        if line.trim().is_empty() {
            continue;
        }
        let request = serde_json::from_str::<Value>(&line).map_err(|source| {
            StdioAdapterError::InvalidJson {
                line: line_number,
                source,
            }
        })?;
        let response =
            host.handle_json(request)
                .map_err(|source| StdioAdapterError::InvalidHostRequest {
                    line: line_number,
                    source,
                })?;
        serde_json::to_writer(&mut writer, &response).map_err(StdioAdapterError::Serialize)?;
        writer.write_all(b"\n").map_err(StdioAdapterError::Write)?;
        writer.flush().map_err(StdioAdapterError::Write)?;
    }
    Ok(())
}

fn read_bounded_json_line<R: BufRead>(
    reader: &mut R,
    max_bytes: usize,
    line: usize,
) -> Result<Option<String>, StdioAdapterError> {
    let max_raw_bytes = max_bytes
        .checked_add(1)
        .ok_or(StdioAdapterError::RecordTooLarge { line, max_bytes })?;
    let mut record = Vec::new();

    loop {
        let buffer = reader.fill_buf().map_err(StdioAdapterError::Read)?;
        if buffer.is_empty() {
            if record.is_empty() {
                return Ok(None);
            }
            if record.len() > max_bytes {
                return Err(StdioAdapterError::RecordTooLarge { line, max_bytes });
            }
            return strict_utf8_line(record).map(Some);
        }

        if let Some(newline) = buffer.iter().position(|byte| *byte == b'\n') {
            let part = &buffer[..newline];
            let total_raw = record.len().saturating_add(part.len());
            let ends_cr = part.last().copied().or_else(|| record.last().copied()) == Some(b'\r');
            let payload_bytes = total_raw.saturating_sub(usize::from(ends_cr));
            if payload_bytes > max_bytes {
                return Err(StdioAdapterError::RecordTooLarge { line, max_bytes });
            }
            record.extend_from_slice(part);
            reader.consume(newline + 1);
            if ends_cr {
                record.pop();
            }
            return strict_utf8_line(record).map(Some);
        }

        let total_raw = record.len().saturating_add(buffer.len());
        if total_raw > max_raw_bytes {
            return Err(StdioAdapterError::RecordTooLarge { line, max_bytes });
        }
        if total_raw == max_raw_bytes && buffer.last().copied() != Some(b'\r') {
            return Err(StdioAdapterError::RecordTooLarge { line, max_bytes });
        }

        record.extend_from_slice(buffer);
        let consumed = buffer.len();
        reader.consume(consumed);
    }
}

fn strict_utf8_line(bytes: Vec<u8>) -> Result<String, StdioAdapterError> {
    String::from_utf8(bytes)
        .map_err(|error| StdioAdapterError::Read(io::Error::new(io::ErrorKind::InvalidData, error)))
}

#[derive(Debug)]
struct StdioCliError(String);

impl fmt::Display for StdioCliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Error for StdioCliError {}

#[derive(Debug)]
enum StdioAdapterError {
    Read(io::Error),
    RecordTooLarge {
        line: usize,
        max_bytes: usize,
    },
    InvalidJson {
        line: usize,
        source: serde_json::Error,
    },
    InvalidHostRequest {
        line: usize,
        source: ReadOnlyJsonToolHostProtocolError,
    },
    Serialize(serde_json::Error),
    Write(io::Error),
}

impl fmt::Display for StdioAdapterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read(error) => write!(f, "failed to read stdin: {error}"),
            Self::RecordTooLarge { line, max_bytes } => write!(
                f,
                "stdin JSON record on line {line} exceeded the {max_bytes}-byte transport limit"
            ),
            Self::InvalidJson { line, source } => {
                write!(f, "invalid JSON on stdin line {line}: {source}")
            }
            Self::InvalidHostRequest { line, source } => {
                write!(f, "invalid host request on stdin line {line}: {source}")
            }
            Self::Serialize(error) => write!(f, "failed to serialize host response: {error}"),
            Self::Write(error) => write!(f, "failed to write stdout: {error}"),
        }
    }
}

impl Error for StdioAdapterError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Read(error) | Self::Write(error) => Some(error),
            Self::RecordTooLarge { .. } => None,
            Self::InvalidJson { source, .. } => Some(source),
            Self::InvalidHostRequest { source, .. } => Some(source),
            Self::Serialize(error) => Some(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::convert::Infallible;
    use std::io::Cursor;
    use world_agent_tools::{
        JsonToolInvocationError, ReadOnlyJsonTool, ReadOnlyJsonToolDescriptor,
    };

    struct EchoTool;

    impl ReadOnlyJsonTool for EchoTool {
        type ExecutorError = Infallible;

        fn json_descriptor(&self) -> ReadOnlyJsonToolDescriptor {
            ReadOnlyJsonToolDescriptor {
                name: "world.echo",
                description: "Echo input for stdio framing tests.",
                read_only: true,
                input_schema: serde_json::json!({"type": "object"}),
            }
        }

        fn invoke_json(
            &mut self,
            input: Value,
        ) -> Result<Value, JsonToolInvocationError<Self::ExecutorError>> {
            Ok(input)
        }
    }

    fn echo_host() -> ReadOnlyJsonToolHost<Infallible> {
        let mut registry = ReadOnlyJsonToolRegistry::<Infallible>::new();
        registry.register(EchoTool).unwrap();
        ReadOnlyJsonToolHost::new(registry)
    }

    #[test]
    fn bounded_reader_preserves_lf_crlf_eof_and_utf8_semantics() {
        let mut lf = Cursor::new(b"abcd\nrest");
        assert_eq!(
            read_bounded_json_line(&mut lf, 4, 1).unwrap(),
            Some("abcd".into())
        );
        assert_eq!(
            read_bounded_json_line(&mut lf, 4, 2).unwrap(),
            Some("rest".into())
        );

        let mut crlf = Cursor::new(b"abcd\r\n");
        assert_eq!(
            read_bounded_json_line(&mut crlf, 4, 1).unwrap(),
            Some("abcd".into())
        );

        let mut eof = Cursor::new(b"abcd");
        assert_eq!(
            read_bounded_json_line(&mut eof, 4, 1).unwrap(),
            Some("abcd".into())
        );

        let mut lone_cr = Cursor::new(b"abc\r");
        assert_eq!(
            read_bounded_json_line(&mut lone_cr, 4, 1).unwrap(),
            Some("abc\r".into())
        );

        let mut utf8 = Cursor::new("éé\n".as_bytes());
        assert_eq!(
            read_bounded_json_line(&mut utf8, 4, 1).unwrap(),
            Some("éé".into())
        );
    }

    #[test]
    fn bounded_reader_handles_chunk_splits_and_multiple_records() {
        let input = Cursor::new(b"one\ntwo\nthree");
        let mut reader = BufReader::with_capacity(2, input);
        assert_eq!(
            read_bounded_json_line(&mut reader, 5, 1).unwrap(),
            Some("one".into())
        );
        assert_eq!(
            read_bounded_json_line(&mut reader, 5, 2).unwrap(),
            Some("two".into())
        );
        assert_eq!(
            read_bounded_json_line(&mut reader, 5, 3).unwrap(),
            Some("three".into())
        );
        assert_eq!(read_bounded_json_line(&mut reader, 5, 4).unwrap(), None);
    }

    #[test]
    fn bounded_reader_rejects_oversize_and_invalid_utf8() {
        let mut newline = Cursor::new(b"abcde\n");
        assert!(matches!(
            read_bounded_json_line(&mut newline, 4, 7),
            Err(StdioAdapterError::RecordTooLarge {
                line: 7,
                max_bytes: 4
            })
        ));

        let mut eof = Cursor::new(b"abcde");
        assert!(matches!(
            read_bounded_json_line(&mut eof, 4, 8),
            Err(StdioAdapterError::RecordTooLarge {
                line: 8,
                max_bytes: 4
            })
        ));

        let mut invalid = Cursor::new(vec![0xff, b'\n']);
        assert!(matches!(
            read_bounded_json_line(&mut invalid, 4, 9),
            Err(StdioAdapterError::Read(error)) if error.kind() == io::ErrorKind::InvalidData
        ));
    }

    struct PanicAfterFirstFill {
        bytes: Vec<u8>,
        consumed: bool,
    }

    impl std::io::Read for PanicAfterFirstFill {
        fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
            unreachable!("BufRead path should not call Read::read")
        }
    }

    impl BufRead for PanicAfterFirstFill {
        fn fill_buf(&mut self) -> io::Result<&[u8]> {
            assert!(
                !self.consumed,
                "overflow reader waited for another fill/EOF"
            );
            Ok(&self.bytes)
        }

        fn consume(&mut self, amount: usize) {
            self.consumed = amount > 0;
        }
    }

    #[test]
    fn no_newline_overflow_fails_before_waiting_for_eof() {
        let mut reader = PanicAfterFirstFill {
            bytes: b"abcde".to_vec(),
            consumed: false,
        };
        assert!(matches!(
            read_bounded_json_line(&mut reader, 4, 1),
            Err(StdioAdapterError::RecordTooLarge {
                line: 1,
                max_bytes: 4
            })
        ));
        assert!(!reader.consumed);
    }

    #[test]
    fn pending_cr_headroom_accepts_exact_limit_crlf() {
        let input = Cursor::new(b"abcd\r\n");
        let mut reader = BufReader::with_capacity(5, input);
        assert_eq!(
            read_bounded_json_line(&mut reader, 4, 1).unwrap(),
            Some("abcd".into())
        );
    }

    #[test]
    fn physical_line_numbers_include_ignored_blank_lines() {
        let mut host = echo_host();
        let input = Cursor::new(b"\n   \n{not-json}\n");
        let mut output = Vec::new();
        let error = serve_json_lines_with_limit(&mut host, input, &mut output, 64).unwrap_err();
        assert!(matches!(
            error,
            StdioAdapterError::InvalidJson { line: 3, .. }
        ));
        assert!(output.is_empty());
    }

    #[test]
    fn first_complete_request_is_flushed_before_second_record_overflow() {
        let mut host = echo_host();
        let first = b"{\"op\":\"list-tools\"}\n";
        let mut input = first.to_vec();
        input.extend_from_slice(&[b'x'; 65]);
        let mut output = Vec::new();
        let error = serve_json_lines_with_limit(&mut host, Cursor::new(input), &mut output, 64)
            .unwrap_err();
        assert!(matches!(
            error,
            StdioAdapterError::RecordTooLarge {
                line: 2,
                max_bytes: 64
            }
        ));
        let output = String::from_utf8(output).unwrap();
        let mut lines = output.lines();
        let first_response: Value = serde_json::from_str(lines.next().unwrap()).unwrap();
        assert_eq!(first_response["type"], "catalog");
        assert!(lines.next().is_none());
    }

    #[test]
    fn production_framing_uses_fixed_bounded_reader() {
        let source = include_str!("main.rs");
        let production = source.split("#[cfg(test)]").next().unwrap();
        assert!(production.contains("MAX_TOOL_REQUEST_BYTES"));
        assert!(production
            .contains("serve_json_lines_with_limit(host, reader, writer, MAX_TOOL_REQUEST_BYTES)"));
        assert!(!production.contains("reader.lines().enumerate()"));
    }
    #[test]
    fn framing_skips_blank_lines_and_emits_one_json_line_per_request() {
        let mut registry = ReadOnlyJsonToolRegistry::<Infallible>::new();
        registry.register(EchoTool).unwrap();
        let mut host = ReadOnlyJsonToolHost::new(registry);
        let input = Cursor::new(
            b"\n{\"op\":\"invoke\",\"call_id\":\"c1\",\"tool\":\"world.echo\",\"input\":{\"v\":1}}\n{\"op\":\"list-tools\"}\n",
        );
        let mut output = Vec::new();
        serve_json_lines(&mut host, input, &mut output).unwrap();
        let lines = String::from_utf8(output)
            .unwrap()
            .lines()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        assert_eq!(lines.len(), 2);
        let first: Value = serde_json::from_str(&lines[0]).unwrap();
        let second: Value = serde_json::from_str(&lines[1]).unwrap();
        assert_eq!(first["call_id"], "c1");
        assert_eq!(first["output"]["v"], 1);
        assert_eq!(second["type"], "catalog");
    }
}
