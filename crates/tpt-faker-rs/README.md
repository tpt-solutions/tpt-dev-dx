# tpt-faker-rs

[![crates.io](https://img.shields.io/crates/v/tpt-faker-rs.svg)](https://crates.io/crates/tpt-faker-rs)
[![docs.rs](https://docs.rs/tpt-faker-rs/badge.svg)](https://docs.rs/tpt-faker-rs)
[![CI](https://github.com/tpt-solutions/tpt-dev-dx/actions/workflows/ci.yml/badge.svg)](https://github.com/tpt-solutions/tpt-dev-dx/actions)

Strongly-typed, realistic mock data generation for Rust.

Generates data that passes real-world constraints: Luhn-valid credit card numbers, valid ISO 8601 dates, realistic names, and more.

## Quick Start

```toml
[dev-dependencies]
tpt-faker-rs = "0.1"
```

```rust
use tpt_faker_rs::Fake;

#[derive(Fake, Debug)]
struct User {
    #[fake(kind = "name")]
    name: String,

    #[fake(kind = "email")]
    email: String,

    #[fake(kind = "luhn_card")]
    credit_card: String,

    #[fake(range = "18..=99")]
    age: u8,

    #[fake(kind = "iso_date")]
    joined: String,
}

let user = User::fake();
println!("{user:?}");
// User { name: "Alice Smith", email: "foxtrot42@example.com",
//         credit_card: "4532015112830366", age: 34, joined: "2019-07-22" }
```

## Available generators

| `kind` value | Output |
|---|---|
| `name` | Full name (first + last) |
| `first_name` | First name |
| `last_name` | Last name |
| `email` | Email address |
| `username` | Username with random suffix |
| `url` | HTTPS URL |
| `ipv4` | IPv4 address |
| `ipv6` | IPv6 address |
| `uuid` | UUID v4 string |
| `luhn_card` | 16-digit Luhn-valid card number |
| `iso_date` | `YYYY-MM-DD` date |
| `iso_datetime` | ISO 8601 datetime with `Z` suffix |
| `word` | Single word |
| `sentence` | A sentence of 5–12 words |
| `paragraph` | 3–6 sentences |

Use `#[fake(range = "lo..=hi")]` for bounded integers.

## Using generators directly

```rust
use tpt_faker_rs::gen;

let card = gen::luhn_card();   // "4532015112830366"
let id = gen::uuid();          // "550e8400-e29b-41d4-a716-446655440000"
let date = gen::iso_date();    // "2023-04-15"
```

## License

MIT OR Apache-2.0
