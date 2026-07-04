# finetype-mcp

MCP (Model Context Protocol) server for
[FineType](https://github.com/meridian-online/finetype) — exposes semantic
type profiling as MCP tools so agents and MCP-capable clients can profile
tabular data (which columns are ISINs, NAICS codes, timestamps, coordinates?)
and generate typed schemas.

Most users run this through the `finetype` CLI (`finetype mcp` starts the
server over stdio):
[installation instructions](https://github.com/meridian-online/finetype#installation).

This crate is primarily a component of the CLI; the library API carries no
stability promises. See
[`finetype-core`](https://crates.io/crates/finetype-core) for the taxonomy
and validators, and
[`finetype-model`](https://crates.io/crates/finetype-model) for the
inference pipeline.

License: MIT
