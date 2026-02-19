// Spinner utility - simple implementation
#[allow(dead_code)]
pub struct Spinner {
    frames: Vec<&'static str>,
    index: usize,
}

impl Spinner {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            frames: vec![
                "▰▱▱▱▱▱▱",
                "▰▰▱▱▱▱▱",
                "▰▰▰▱▱▱▱",
                "▰▰▰▰▱▱▱",
                "▰▰▰▰▰▱▱",
                "▰▰▰▰▰▰▱",
                "▰▰▰▰▰▰▰",
                "▱▱▱▱▱▱▱",
            ],
            index: 0,
        }
    }

    #[allow(dead_code)]
    pub fn next(&mut self) -> &'static str {
        let frame = self.frames[self.index];
        self.index = (self.index + 1) % self.frames.len();
        frame
    }
}

impl Default for Spinner {
    fn default() -> Self {
        Self::new()
    }
}

