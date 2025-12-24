# Configuration Guide (config.json)

Example:

```json
{
  "neo4j_uri": "bolt://localhost:7687",
  "neo4j_user": "neo4j",
  "neo4j_password": "your_password_here",
  "neo4j_database": null,
  "batch_size": 1000
}
```

Field descriptions:

- `neo4j_uri`: Neo4j Bolt address (required), e.g. `bolt://localhost:7687`
- `neo4j_user`: Username (required)
- `neo4j_password`: Password (required)
- `neo4j_database`: Database name (optional)
  - `null`/omitted: Use default database
  - Specified string: Connect to specific database (requires Neo4j Enterprise or Aura with multi-database support)
- `batch_size`: Number of files processed per batch during directory batch import (optional, default 1000)