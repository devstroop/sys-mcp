/// Server configuration parsed from CLI arguments.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub web_preview: bool,
    pub debug: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            web_preview: true,
            debug: false,
        }
    }
}

impl ServerConfig {
    pub fn from_args() -> Self {
        let mut config = Self::default();
        let args: Vec<String> = std::env::args().collect();
        let mut i = 1;

        while i < args.len() {
            match args[i].as_str() {
                "--no-web-preview" => config.web_preview = false,
                "--debug" => config.debug = true,
                _ => {} // silently ignore unknown args
            }
            i += 1;
        }

        config
    }
}
