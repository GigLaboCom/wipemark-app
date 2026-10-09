//! `--flag value` pairs, nothing more: the bench is a development tool.

pub struct Args {
    pub mode: String,
    pairs: Vec<(String, String)>,
}

impl Args {
    pub fn parse() -> Args {
        let mut raw = std::env::args().skip(1);
        let mode = raw.next().unwrap_or_else(|| {
            eprintln!("{}", crate::USAGE);
            std::process::exit(2);
        });
        let mut pairs = Vec::new();
        while let Some(flag) = raw.next() {
            if !flag.starts_with("--") {
                panic!("expected a --flag, got {flag:?}");
            }
            let value = raw.next().unwrap_or_else(|| panic!("{flag} needs a value"));
            pairs.push((flag, value));
        }
        Args { mode, pairs }
    }

    /// `mode` and its `--flag value` pairs, as a test hands them.
    #[cfg(test)]
    pub fn of(mode: &str, pairs: &[(&str, &str)]) -> Args {
        Args {
            mode: mode.to_owned(),
            pairs: pairs
                .iter()
                .map(|(flag, value)| (flag.to_string(), value.to_string()))
                .collect(),
        }
    }

    pub fn value(&self, flag: &str) -> Option<String> {
        self.pairs
            .iter()
            .rev()
            .find(|(f, _)| f == flag)
            .map(|(_, v)| v.clone())
    }

    pub fn required(&self, flag: &str) -> String {
        self.value(flag)
            .unwrap_or_else(|| panic!("{flag} is required\n\n{}", crate::USAGE))
    }

    /// A comma-separated list; empty when the flag is absent.
    pub fn list(&self, flag: &str) -> Vec<String> {
        self.value(flag)
            .map(|v| {
                v.split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default()
    }
}
