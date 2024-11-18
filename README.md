# Ekv

Ekv is a distributed key-value store, used as a cache, database, and storage engine.

## Architecture

![topology][topology]

See [design doc][design-doc] for more details.

[topology]: ./docs/img/topology.drawio.svg
[design-doc]: ./docs/design.md

## Quick start

1. Build

```sh
make build
```

2. Deploy a cluster

```sh
bash scripts/bootstrap.sh setup
```

3. Verify

```sh
cargo run -- shell
```

Run and enjoy it.
