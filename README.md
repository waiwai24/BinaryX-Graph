# BinaryX-Graph

Cross-binary analysis data importer written in Rust, focused on importing pre-analyzed binary data into Neo4j graph database for graph analysis.

## Features

- **Fulltext Search**: Built-in Lucene-based fulltext index for fast substring/keyword search in strings
- **Neo4j Integration**: Native neo4rs driver for direct Neo4j database connection
- **Modern CLI**: Command-line interface based on clap with multiple output formats
- **Flexible Configuration**: JSON configuration file support
- **Single Deployment**: Compiled as a single executable with no runtime dependencies
- **Call Path Analysis**: Complete function call chain and execution order analysis, solving the "who calls whom and in what order" problem
- **Upward Call Chain**: Trace function call sources to build complete call context
- **Recursion Detection**: Automatically identify direct and indirect recursive calls to prevent infinite loops
- **Execution Order**: Precise execution timing analysis based on call addresses

## Documentation

- Usage Guide: [USAGE.md](docs/USAGE.md)
- Configuration Guide: [CONFIG.md](docs/CONFIG.md)

## Changelog

See [CHANGELOG.md](/docs/CHANGELOG.md)

## TODO

- [ ] Web UI interface development
- [ ] Native Cypher language query interface

## Contact

 **Issues**: [GitHub Issues](https://github.com/waiwai24/BinaryX-Graph/issues/new)

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
