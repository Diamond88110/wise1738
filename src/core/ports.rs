#[derive(Clone)]
pub struct Ports {
    pub ports: Vec<u16>,
}

impl Ports {
    /// All ports: 1–65535
    pub fn all() -> Self {
        Self {
            ports: (1..=65535).collect(),
        }
    }

    /// Single port: 80
    pub fn single(port: u16) -> Self {
        Self {
            ports: vec![port],
        }
    }

    /// Multiple ports: 22,80,443
    pub fn multiple(list: Vec<u16>) -> Self {
        Self {
            ports: list,
        }
    }

    /// Range: 20–200, 300–500
    pub fn range(start: u16, end: u16) -> Self {
        let (a, b) = if start <= end { (start, end) } else { (end, start) };
        Self {
            ports: (a..=b).collect(),
        }
    }

    /// Top common ports (starter set)
    pub fn top_basic() -> Self {
        Self {
            ports: vec![
                21, 22, 23, 25, 53,
                80, 110, 139, 143,
                443, 445, 3306,
                3389, 5432, 6379,
                8080, 8443,
            ],
        }
    }
}
