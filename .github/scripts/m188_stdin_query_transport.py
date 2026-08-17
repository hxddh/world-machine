from pathlib import Path

path = Path("crates/world-cli/src/main.rs")
text = path.read_text()

old = '''use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
'''
new = '''use std::fmt;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
'''
if text.count(old) != 1:
    raise SystemExit("import marker missing")
text = text.replace(old, new, 1)

old = '''        Command::EvidenceQuery(path, request) => {
            println!("{}", evidence_query_json_report(&path, &request)?)
        }
        Command::EvidenceCompareQuery(left, right, request) => {
            println!(
                "{}",
                evidence_compare_query_json_report(&left, &right, &request)?
            )
        }
'''
new = '''        Command::EvidenceQuery(path, request) => {
            let request = read_query_request(&request)?;
            println!("{}", evidence_query_json_report(&path, &request)?)
        }
        Command::EvidenceCompareQuery(left, right, request) => {
            let request = read_query_request(&request)?;
            println!(
                "{}",
                evidence_compare_query_json_report(&left, &right, &request)?
            )
        }
'''
if text.count(old) != 1:
    raise SystemExit("machine query handler marker missing")
text = text.replace(old, new, 1)

old = '''  world-cli evidence-query <file.world> '<request-json>'\\n\\n\\
  world-cli evidence-compare-query <left.world> <right.world> '<request-json>'\\n\\n\\
'''
new = '''  world-cli evidence-query <file.world> <request-json|->\\n\\n\\
  world-cli evidence-compare-query <left.world> <right.world> <request-json|->\\n\\n\\
'''
if text.count(old) != 1:
    raise SystemExit("usage command marker missing")
text = text.replace(old, new, 1)

old = '''evidence-query  Execute an EvidenceQueryRequest JSON document and emit a JSON status envelope.\\n\\
evidence-compare-query  Execute an EvidenceComparisonRequest JSON document and emit a JSON status envelope.\\n\\
list-packs  List World Packs this build can create and restore."
'''
new = '''evidence-query  Execute an EvidenceQueryRequest JSON document and emit a JSON status envelope. Use - to read JSON from stdin.\\n\\
evidence-compare-query  Execute an EvidenceComparisonRequest JSON document and emit a JSON status envelope. Use - to read JSON from stdin.\\n\\
list-packs  List World Packs this build can create and restore."
'''
if text.count(old) != 1:
    raise SystemExit("usage description marker missing")
text = text.replace(old, new, 1)

marker = '''fn load_archive(path: &Path) -> Result<WorldArchive, Box<dyn Error>> {
'''
helper = '''fn read_query_request(request: &str) -> Result<String, Box<dyn Error>> {
    if request != "-" {
        return Ok(request.to_owned());
    }

    let mut json = String::new();
    io::stdin().read_to_string(&mut json)?;
    Ok(json)
}

'''
if text.count(marker) != 1:
    raise SystemExit("request helper insertion marker missing")
text = text.replace(marker, helper + marker, 1)

path.write_text(text)
