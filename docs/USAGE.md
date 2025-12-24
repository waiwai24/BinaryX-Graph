# Usage Documentation (binaryx CLI)

This document describes the actual usage of `binaryx` (import/query/database maintenance) and provides recommended workflows for batch sample processing.

## 0. Command Format and Help

Basic format:

```bash
binaryx [-c config.json] <command> <subcommand> [options]
```

- `-c/--config`: Configuration file path; defaults to `config.json` in current directory if not specified
- View help:
  - `binaryx --help`
  - `binaryx import --help`
  - `binaryx query --help`
  - `binaryx database --help`

## 1. Typical Workflows

### 1.1 First-time Import

```bash
./binaryx -c config.json database init
./binaryx -c config.json import directory ./analysis_data --pattern "*.json"
```

If you upgraded versions, changed the graph model (e.g., global deduplication/relationship property changes), it's recommended to clear the database and re-import:

```bash
./binaryx -c config.json database clear --confirm
./binaryx -c config.json database init
./binaryx -c config.json import directory ./analysis_data --pattern "*.json"
```

### 1.2 Batch Import (Large-scale Samples)

```bash
./binaryx -c config.json import json ./analysis_data/one.json
./binaryx -c config.json import directory ./analysis_data --pattern "*.json" --batch-size 200 --no-validate
```

Recommendations:

- `--no-validate`: Enable after confirming data stability for faster performance
- `--batch-size`: Control "files per batch" (not Neo4j batch writes); typical range `100~1000`

Limitations:

- `import directory` only scans "single-level directories" (does not recurse into subdirectories). If your data is stored in layers, repeat import for each subdirectory, or flatten JSON files to the same directory first.

## 2. Database (Database Maintenance)

### 2.1 Initialize Schema

```bash
./binaryx -c config.json database init
```

Purpose:

- Create constraints/indexes
- Create string fulltext index `string_value_fulltext` (used for `query strings`)
- Also performs Neo4j connectivity verification

### 2.2 Clear Database

```bash
./binaryx -c config.json database clear
```

Skip interactive confirmation:

```bash
./binaryx -c config.json database clear --confirm
```

### 2.3 Statistics

```bash
./binaryx -c config.json database stats
```

### 2.4 Export

```bash
./binaryx -c config.json database export backup.json
```

## 3. Import

### 3.1 Import Single File

```bash
./binaryx -c config.json import json analysis.json
```

Common uses:

- Verify if data format is acceptable
- Locate import errors for specific samples

Parameters:

- `--no-validate`: Skip data validation (faster)

### 3.2 Import Directory

```bash
./binaryx -c config.json import directory ./analysis_data --pattern "*.json" --batch-size 500
```

Parameters:

- `--pattern`: Filename matching (simplified matching rules)
  - `"*"` / `"*.*"`: Match all
  - `"*.json"`: By suffix
  - `"prefix*"`: By prefix
  - `"*suffix"`: By suffix (string suffix)
  - Others: Exact match
- `--batch-size`: Files per batch
- `--no-validate`: Skip validation

Output:

- Each file prints import progress and statistics for that file
- After directory import completes, summarizes success count and errors (shows up to first 10 errors)

## 4. Query

### 4.1 query strings (String Fulltext / Substring Search)

```bash
./binaryx -c config.json query strings --pattern "password"
```

Filter by binary:

```bash
./binaryx -c config.json query strings --pattern "password" --binary "malware.exe"
```

Default `--pattern` automatically converts to Lucene wildcard query (more like "substring" experience):

- Input: `Pay Bitcoin`
- Actual: `*Pay* AND *Bitcoin*`

Use raw to pass native Lucene query (phrase/boolean):

```bash
./binaryx -c config.json query strings --raw --pattern "\"Pay Bitcoin\""
./binaryx -c config.json query strings --raw --pattern "ransom* AND (bitcoin OR wallet)"
```

Parameters:

- `--pattern`: Search content (default will be converted)
- `--raw`: Disable auto-conversion, use Lucene query syntax directly
- `--binary`: Filter specific binary (`Binary.filename CONTAINS <binary>` or `Binary.hash == <binary>`)
- `--limit`: Maximum results to return (default 100)
- `--format`: `table` / `json`

Output fields (table mode):

- `Score`: Fulltext relevance score
- `Samples`: Number of samples containing this string (deduplicated count)
- `Binaries`: Hit sample preview (up to 5)
- `Value`: String content (with simple truncation and newline escaping)

Dependencies:

- Must have executed `./binaryx -c config.json database init` to create fulltext index, otherwise will report "index does not exist"

### 4.2 query functions (Function Query)

```bash
./binaryx -c config.json query functions --pattern "CreateFile"
./binaryx -c config.json query functions --pattern "main" --binary "sample.exe"
```

Parameters:

- `--pattern`: Match function name or UID substring
- `--binary`: Filter specific binary (`filename CONTAINS` or `hash ==`)
- `--limit`: Maximum results to display (default 100)
- `--format`: `table` / `json`

### 4.3 query binary (Binary Information)

```bash
./binaryx -c config.json query binary --binary-name "sample.exe"
./binaryx -c config.json query binary --binary-name "fd8c2f0d..." --format json
```

Note:

- `--binary-name` supports filename substring matching or hash exact matching

### 4.4 query callgraph (Call Graph)

```bash
./binaryx -c config.json query callgraph CreateFileA --max-depth 2
./binaryx -c config.json query callgraph CreateFileA --binary "sample.exe" --max-depth 2
```

Parameters:

- `--binary`: Filter binary
- `--max-depth`: Depth (larger = slower)
- `--show-callees/--show-callers`: Show only one side (both shown if not specified)
- `--format`: `table` / `json`

### 4.5 query call-path (Call Path/Context)

```bash
./binaryx -c config.json query call-path "main" --binary "sample.exe" --show-context
```

Common parameters:

- `--max-depth`: Default 5, larger = slower
- `--show-paths`: Output path structure
- `--show-sequences`: Output call sequences sorted by call offset
- `--show-recursive`: Recursion detection
- `--show-upward`: Upward call chain
- `--show-context`: Comprehensive context analysis

### 4.6 query xrefs (Cross References)

```bash
./binaryx -c config.json query xrefs 0x401000 --binary "sample.exe"
```

Recommendation:

- Try to include `--binary` for address queries to avoid result ambiguity from shared nodes after cross-sample aggregation

## 5. Common Issues

### 5.1 `query strings` Slow/Stuck

Priority checks:

1. Whether `database init` was executed (whether fulltext index exists)
2. Whether `--limit` is set too large (recommend starting from 20/50)
3. Whether Neo4j server CPU/IO is saturated

### 5.2 Why don't `String` nodes have `address` property?

For cross-sample global deduplication, `String` nodes only represent "content". Address is "location in a specific binary", belonging to relationship properties:

- `(Binary)-[:CONTAINS_STRING {address}]->(String)`

### 5.3 Why don't imported function nodes have `address` property?

Import API nodes are globally shared resources, different samples have different IAT addresses, so addresses are stored in relationship properties:

- `(Binary)-[:IMPORTS {address}]->(Function{type=Import})`