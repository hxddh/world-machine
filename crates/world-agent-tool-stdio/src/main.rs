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

fn main() -> Result<(), Box<dyn Error>> {
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
    mut writer: W,
) -> Result<(), StdioAdapterError>
where
    E: fmt::Display,
    R: BufRead,
    W: Write,
{
    for (index, line) in reader.lines().enumerate() {
        let line_number = index + 1;
        let line = line.map_err(StdioAdapterError::Read)?;
        if line.trim().is_empty() {
            continue;
        }
        let request = serde_json::from_str::<Value>(&line).map_err(|source| {
            StdioAdapterError::InvalidJson {
                line: line_number,
                source,
            }
        })?;
        let response = host
            .handle_json(request)
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
