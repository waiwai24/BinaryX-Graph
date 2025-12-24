# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **Fulltext Search** capability for strings
  - Neo4j Lucene-based fulltext index (`string_value_fulltext`)
  - CLI command: `binaryx query strings --pattern "ransomware"`
  - Supports wildcard queries, boolean operators, and phrase matching
  - Returns relevance scores and sample count aggregation
- **New data model** `StringSearchHit` with relevance scoring
  - Fields: `uid`, `value`, `score`, `sample_count`
- **New relationship types**
  - `IMPORTS` (Binary → Function) with `address` property for import locations
  - `CONTAINS_STRING` (Binary → String) with `address` property for string locations
  - `IMPORTS_LIBRARY` (Binary → Library) to track library dependencies

### Changed

- **BREAKING**: Import Function UID format changed
  - Old: `imp:{binary_hash}:{library}:{name}` (per-binary)
  - New: `imp:{library}:{name}` (global)
- **BREAKING**: String UID format changed
  - Old: `str:{binary_hash}:{DefaultHasher(content)}` (per-binary, unstable)
  - New: `str:{SHA256(content)}` (global, stable)
- **BREAKING**: Import function `address` moved from node property to `IMPORTS` relationship property
- **BREAKING**: String `address` moved from node property to `CONTAINS_STRING` relationship property
- **Schema changes**:
  - Added fulltext index on `String.value`
  - All unique constraints remain (Binary.hash, Function.uid, String.uid, Library.name)
- **Query logic updated** to support new `[:CONTAINS|IMPORTS]` relationship patterns

### Fixed

- String UID stability issue - SHA-256 guarantees identical UIDs across Rust versions
- Import function deduplication - same API across binaries now shares single node

## [0.1.0] - 2025-12-24

* First release.

## How to Update This Changelog

When making changes to the project:

1. Add new entries under the "Unreleased" section
2. Use appropriate categories: Added, Changed, Deprecated, Removed, Fixed, Security
3. When releasing a version:
   * Move "Unreleased" entries to a new version section
   * Add release date
   * Create a new empty "Unreleased" section
