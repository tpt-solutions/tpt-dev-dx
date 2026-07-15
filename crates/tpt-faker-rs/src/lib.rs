pub use tpt_faker_rs_derive::Fake;

/// The core trait implemented by types that can generate fake instances of themselves.
pub trait Fake: Sized {
    fn fake() -> Self;
}

/// Low-level data generators — called by the derive macro and available directly.
pub mod gen {
    use rand::Rng;

    thread_local! {
        static RNG: std::cell::RefCell<rand::rngs::ThreadRng> =
            std::cell::RefCell::new(rand::thread_rng());
    }

    fn with_rng<T>(f: impl FnOnce(&mut rand::rngs::ThreadRng) -> T) -> T {
        RNG.with(|r| f(&mut r.borrow_mut()))
    }

    // ── Names ────────────────────────────────────────────────────────────────

    static FIRST_NAMES: &[&str] = &[
        "Alice", "Bob", "Carol", "David", "Eva", "Frank", "Grace", "Henry",
        "Iris", "Jack", "Karen", "Leo", "Mia", "Nathan", "Olivia", "Paul",
        "Quinn", "Rachel", "Sam", "Tara", "Uma", "Victor", "Wendy", "Xander",
        "Yara", "Zoe", "Liam", "Emma", "Noah", "Ava", "Oliver", "Sofia",
        "Elijah", "Mila", "William", "Aria", "James", "Scarlett", "Lucas", "Luna",
    ];

    static LAST_NAMES: &[&str] = &[
        "Smith", "Jones", "Williams", "Brown", "Taylor", "Davies", "Evans",
        "Wilson", "Thomas", "Roberts", "Johnson", "Walker", "Wright", "Thompson",
        "Robinson", "White", "Hughes", "Edwards", "Green", "Hall", "Lewis",
        "Harris", "Clarke", "Patel", "Jackson", "Wood", "Turner", "Martin",
        "Cooper", "Hill", "Ward", "Morris", "Moore", "Clark", "Lee", "King",
        "Baker", "Harrison", "Morgan", "Allen",
    ];

    pub fn first_name() -> String {
        pick(FIRST_NAMES).to_string()
    }

    pub fn last_name() -> String {
        pick(LAST_NAMES).to_string()
    }

    pub fn name() -> String {
        format!("{} {}", first_name(), last_name())
    }

    // ── Internet ─────────────────────────────────────────────────────────────

    static WORDS: &[&str] = &[
        "alpha", "bravo", "charlie", "delta", "echo", "foxtrot", "golf",
        "hotel", "india", "juliet", "kilo", "lima", "mike", "november",
        "oscar", "papa", "quebec", "romeo", "sierra", "tango", "uniform",
        "victor", "whiskey", "xray", "yankee", "zulu", "rust", "cargo",
        "ferris", "crab", "oxide", "async", "trait", "enum", "struct",
    ];

    static DOMAINS: &[&str] = &[
        "example", "test", "demo", "sample", "fake", "mock", "dev",
        "acme", "globex", "initech", "umbrella", "soylent",
    ];

    static TLDS: &[&str] = &["com", "net", "org", "io", "dev", "co"];

    pub fn word() -> String {
        pick(WORDS).to_string()
    }

    pub fn sentence() -> String {
        let n = with_rng(|r| r.gen_range(5..=12));
        let words: Vec<&str> = (0..n).map(|_| *pick(WORDS)).collect();
        let mut s = words.join(" ");
        if let Some(c) = s.get_mut(0..1) {
            c.make_ascii_uppercase();
        }
        s.push('.');
        s
    }

    pub fn paragraph() -> String {
        let n = with_rng(|r| r.gen_range(3..=6));
        (0..n).map(|_| sentence()).collect::<Vec<_>>().join(" ")
    }

    pub fn username() -> String {
        format!("{}{}", pick(WORDS), with_rng(|r| r.gen_range(10u32..=9999)))
    }

    pub fn email() -> String {
        format!("{}@{}.{}", username(), pick(DOMAINS), pick(TLDS))
    }

    pub fn url() -> String {
        format!("https://{}.{}/{}", pick(DOMAINS), pick(TLDS), pick(WORDS))
    }

    pub fn ipv4() -> String {
        with_rng(|r| {
            format!("{}.{}.{}.{}", r.gen::<u8>(), r.gen::<u8>(), r.gen::<u8>(), r.gen::<u8>())
        })
    }

    pub fn ipv6() -> String {
        with_rng(|r| {
            format!(
                "{:04x}:{:04x}:{:04x}:{:04x}:{:04x}:{:04x}:{:04x}:{:04x}",
                r.gen::<u16>(), r.gen::<u16>(), r.gen::<u16>(), r.gen::<u16>(),
                r.gen::<u16>(), r.gen::<u16>(), r.gen::<u16>(), r.gen::<u16>(),
            )
        })
    }

    // ── Identifiers ──────────────────────────────────────────────────────────

    /// Generate a UUID v4 string without any external crate.
    pub fn uuid() -> String {
        with_rng(|r| {
            let mut b = [0u8; 16];
            r.fill(&mut b);
            // Set version bits (4) and variant bits (RFC 4122).
            b[6] = (b[6] & 0x0f) | 0x40;
            b[8] = (b[8] & 0x3f) | 0x80;
            format!(
                "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-\
                 {:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
                b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
                b[8], b[9], b[10], b[11], b[12], b[13], b[14], b[15],
            )
        })
    }

    // ── Payment ──────────────────────────────────────────────────────────────

    /// Generate a 16-digit string that passes the Luhn check.
    pub fn luhn_card() -> String {
        with_rng(|r| {
            // Generate 15 random digits, then compute the Luhn check digit.
            let mut digits: Vec<u8> = (0..15).map(|_| r.gen_range(0..10)).collect();
            let check = luhn_check_digit(&digits);
            digits.push(check);
            digits.iter().map(|d| d.to_string()).collect()
        })
    }

    fn luhn_check_digit(digits: &[u8]) -> u8 {
        let sum: u32 = digits
            .iter()
            .rev()
            .enumerate()
            .map(|(i, &d)| {
                if i % 2 == 0 {
                    let v = d as u32 * 2;
                    if v > 9 { v - 9 } else { v }
                } else {
                    d as u32
                }
            })
            .sum();
        ((10 - (sum % 10)) % 10) as u8
    }

    // ── Dates ────────────────────────────────────────────────────────────────

    pub fn iso_date() -> String {
        with_rng(|r| {
            let year = r.gen_range(1970..=2030u16);
            let month = r.gen_range(1..=12u8);
            let day = r.gen_range(1..=28u8); // safe for all months
            format!("{year:04}-{month:02}-{day:02}")
        })
    }

    pub fn iso_datetime() -> String {
        with_rng(|r| {
            let year = r.gen_range(1970..=2030u16);
            let month = r.gen_range(1..=12u8);
            let day = r.gen_range(1..=28u8);
            let hour = r.gen_range(0..=23u8);
            let min = r.gen_range(0..=59u8);
            let sec = r.gen_range(0..=59u8);
            format!("{year:04}-{month:02}-{day:02}T{hour:02}:{min:02}:{sec:02}Z")
        })
    }

    // ── Numeric ──────────────────────────────────────────────────────────────

    /// Return a random `i64` in the inclusive range `[lo, hi]`.
    pub fn range_i64(lo: i64, hi: i64) -> i64 {
        if lo >= hi {
            return lo;
        }
        with_rng(|r| r.gen_range(lo..=hi))
    }

    // ── Utility ──────────────────────────────────────────────────────────────

    fn pick<T>(slice: &[T]) -> &T {
        with_rng(|r| &slice[r.gen_range(0..slice.len())])
    }
}

// ── Blanket impls for common primitives ──────────────────────────────────────

impl Fake for String {
    fn fake() -> Self { gen::word() }
}

impl Fake for bool {
    fn fake() -> Self { gen::range_i64(0, 1) != 0 }
}

macro_rules! impl_fake_int {
    ($t:ty, $lo:expr, $hi:expr) => {
        impl Fake for $t {
            fn fake() -> Self { gen::range_i64($lo, $hi) as $t }
        }
    };
}

impl_fake_int!(u8,  0, 255);
impl_fake_int!(u16, 0, 65535);
impl_fake_int!(u32, 0, i32::MAX as i64);
impl_fake_int!(u64, 0, i64::MAX);
impl_fake_int!(i8,  -128, 127);
impl_fake_int!(i16, -32768, 32767);
impl_fake_int!(i32, i32::MIN as i64, i32::MAX as i64);
impl_fake_int!(i64, i64::MIN, i64::MAX);
impl_fake_int!(usize, 0, 1000);

impl Fake for f32 {
    fn fake() -> Self { gen::range_i64(-1000, 1000) as f32 }
}
impl Fake for f64 {
    fn fake() -> Self { gen::range_i64(-1000, 1000) as f64 }
}

impl<T: Fake> Fake for Option<T> {
    fn fake() -> Self {
        if gen::range_i64(0, 1) == 0 { None } else { Some(T::fake()) }
    }
}

impl<T: Fake> Fake for Vec<T> {
    fn fake() -> Self {
        let n = gen::range_i64(0, 5) as usize;
        (0..n).map(|_| T::fake()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::gen;

    #[test]
    fn luhn_card_passes_check() {
        for _ in 0..100 {
            let card = gen::luhn_card();
            assert_eq!(card.len(), 16, "card should be 16 digits");
            assert!(card.chars().all(|c| c.is_ascii_digit()));
            assert!(luhn_valid(&card));
        }
    }

    fn luhn_valid(s: &str) -> bool {
        let sum: u32 = s.chars().rev().enumerate().map(|(i, c)| {
            let mut d = c.to_digit(10).unwrap();
            if i % 2 == 1 {
                d *= 2;
                if d > 9 { d -= 9; }
            }
            d
        }).sum();
        sum % 10 == 0
    }

    #[test]
    fn uuid_format() {
        let u = gen::uuid();
        let parts: Vec<&str> = u.split('-').collect();
        assert_eq!(parts.len(), 5);
        assert_eq!(parts[0].len(), 8);
        assert_eq!(parts[1].len(), 4);
        assert_eq!(parts[2].len(), 4);
        assert_eq!(parts[3].len(), 4);
        assert_eq!(parts[4].len(), 12);
        assert_eq!(&parts[2][0..1], "4", "version bit");
    }

    #[test]
    fn iso_date_format() {
        let d = gen::iso_date();
        let parts: Vec<&str> = d.split('-').collect();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0].len(), 4);
        assert_eq!(parts[1].len(), 2);
        assert_eq!(parts[2].len(), 2);
    }

    #[test]
    fn email_contains_at() {
        for _ in 0..10 {
            assert!(gen::email().contains('@'));
        }
    }

    #[test]
    fn ipv4_four_octets() {
        let ip = gen::ipv4();
        assert_eq!(ip.split('.').count(), 4);
    }
}
