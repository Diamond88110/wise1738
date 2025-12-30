#[derive(Clone, Debug)]
pub struct Target {
    pub host: String,
}

impl Target {
    pub fn new(input: &str) -> Self {
        Self {
            host: input.to_string(),
        }
    }
}
