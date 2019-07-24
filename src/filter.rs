use std::cmp::Ordering;
use std::fs::Metadata;

#[derive(Clone, Copy)]
pub enum TimeUnit {
    Second,
    Minute,
    Hour,
    Day,
}

#[derive(Clone, Copy)]
pub enum MetaDataFilter {
    Empty,                        // empty filter, always true
    Size(Ordering, u64),          // file size
    Age(Ordering, u64, TimeUnit), // current - last modified time
    Type(bool),                   // true means dir
}

impl MetaDataFilter {
    pub fn new(pattern: &str) -> Option<Self> {
        #[inline]
        fn to_ordering(c: char) -> Option<Ordering> {
            match c {
                '=' => Some(Ordering::Equal),
                '<' => Some(Ordering::Less),
                '>' => Some(Ordering::Greater),
                _ => None,
            }
        }
        #[inline]
        fn to_unit(c: char) -> Option<TimeUnit> {
            match c {
                's' => Some(TimeUnit::Second),
                'm' => Some(TimeUnit::Minute),
                'h' => Some(TimeUnit::Hour),
                'd' => Some(TimeUnit::Day),
                _ => None,
            }
        }

        if pattern == "" {
            Some(MetaDataFilter::Empty) // shortcut
        } else if pattern.starts_with("size") {
            // remainder
            let mut rem = pattern["size".len()..].chars();

            let ord = to_ordering(rem.next()?)?;
            let size = rem.as_str().parse::<u64>().ok()?;

            Some(MetaDataFilter::Size(ord, size))
        } else if pattern.starts_with("age") {
            let mut rem = pattern["age".len()..].chars();

            let ord = to_ordering(rem.next()?)?;
            let unit = to_unit(rem.clone().last()?)?;
            let rem = rem.as_str();
            let age = rem.split_at(rem.as_bytes().len() - 1).0.parse::<u64>().ok()?;

            Some(MetaDataFilter::Age(ord, age, unit))
        } else if pattern.starts_with("type=") {
            let mut rem = pattern["type=".len()..].chars();
            let dir = match rem.next()? {
                'd' => true,
                'r' => false,
                _ => return None,
            };
            if rem.next().is_none() {
                Some(MetaDataFilter::Type(dir))
            } else {
                None
            }
        } else {
            None
        }
    }

    #[inline]
    pub fn is_valid_pattern(pattern: &str) -> bool {
        Self::new(pattern).is_some()
    }

    #[inline]
    pub fn check(&self, m: &Metadata) -> bool {
        match self {
            &MetaDataFilter::Empty => true,
            _ => self.check_nonempty(m),
        }
    }

    fn check_nonempty(&self, m: &Metadata) -> bool {
        use MetaDataFilter::*;
        match self {
            &Size(ord, size) => m.len().cmp(&size) == ord,
            &Age(ord, age, unit) => match m.modified() {
                Ok(time) => match time.elapsed() {
                    Ok(time) => {
                        let time = time.as_secs();
                        let real_age = match unit {
                            TimeUnit::Second => time,
                            TimeUnit::Minute => time / 60,
                            TimeUnit::Hour => time / 3600,
                            TimeUnit::Day => time / (3600 * 24),
                        };
                        real_age.cmp(&age) == ord
                    }
                    Err(_) => true, // happening in the future
                },
                Err(_) => false, // unsupported platform
            },
            &Type(t) => t == m.is_dir(), // t true means dir
            &Empty => true,
        }
    }
}
