# tpt-temp-fs

[![crates.io](https://img.shields.io/crates/v/tpt-temp-fs.svg)](https://crates.io/crates/tpt-temp-fs)
[![docs.rs](https://docs.rs/tpt-temp-fs/badge.svg)](https://docs.rs/tpt-temp-fs)
[![CI](https://github.com/thirtyfiveparts/tpt-dev-dx/actions/workflows/ci.yml/badge.svg)](https://github.com/thirtyfiveparts/tpt-dev-dx/actions)

Ephemeral, isolated file-system fixtures for integration tests.

`TempDir` creates a unique temporary directory and **guarantees cleanup on drop, even if the test panics**.

## Quick Start

```toml
[dev-dependencies]
tpt-temp-fs = "0.1"
```

```rust
use tpt_temp_fs::TempDir;

#[test]
fn test_config_loader() {
    let dir = TempDir::new().unwrap();

    // Write files directly
    dir.write_file("config.toml", r#"[server]\nport = 8080"#).unwrap();
    dir.create_dir("logs").unwrap();

    // Scaffold a whole tree from YAML
    dir.scaffold_from_str("
      data/users.json: '[{\"id\":1}]'
      data/empty/:
    ").unwrap();

    let config_path = dir.child("config.toml");
    assert!(config_path.exists());

    // `dir` is dropped here → entire tree deleted
}
```

## Features

| Feature | Default | Description |
|---------|---------|-------------|
| `scaffold` | ✓ | Enable `scaffold_from_str()` (requires serde_json + serde_yaml) |

Disable with `default-features = false` for a zero-dependency build.

## License

MIT OR Apache-2.0
