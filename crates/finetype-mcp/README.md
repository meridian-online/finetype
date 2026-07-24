# finetype-mcp

Semantic type profiling for
[FineType](https://github.com/meridian-online/finetype) — datapackage and
JSON-schema emitters, taxonomy resources, and tool request/response types for
profiling tabular data (which columns are ISINs, NAICS codes, timestamps,
coordinates?) and generating typed schemas.

## The MCP server role is deprecated

Running an MCP server from this crate is **deprecated**. The MCP server role is
superseded by arcform's `arc mcp` entrypoint — point MCP clients there instead.
The `FineTypeServer::new` / `serve_stdio` lifecycle carries `#[deprecated]`
attributes; the `finetype mcp` subcommand still works for now but will be
removed.

This crate is **retained for its library types** (the datapackage / JSON-schema
emitters and taxonomy resources), which remain supported.

This crate is primarily a component of the CLI; the library API carries no
stability promises. See
[`finetype-core`](https://crates.io/crates/finetype-core) for the taxonomy
and validators, and
[`finetype-model`](https://crates.io/crates/finetype-model) for the
inference pipeline.

License: MIT
