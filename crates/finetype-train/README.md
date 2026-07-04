# finetype-train

Training infrastructure for
[FineType](https://github.com/meridian-online/finetype) — Candle-based
training for the Sense-stage multi-branch classifier and supporting models.

**Internal support crate.** It is published only because `finetype-cli`
lists it as an optional dependency (the `train` feature) and cargo requires
every listed dependency to exist on the registry. It carries no API
stability promises; training runs happen inside the
[FineType workspace](https://github.com/meridian-online/finetype) with its
data pipelines, not against this crate standalone.

For the usable surfaces, see
[`finetype-core`](https://crates.io/crates/finetype-core) (taxonomy +
validators), [`finetype-model`](https://crates.io/crates/finetype-model)
(inference), and the `finetype` CLI
([installation](https://github.com/meridian-online/finetype#installation)).

License: MIT
