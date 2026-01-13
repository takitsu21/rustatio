use serde::Deserialize;

/// Commands that can be sent via stdin in JSON mode
#[derive(Debug, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum InputCommand {
    /// Pause the faker
    Pause,

    /// Resume the faker
    Resume,

    /// Stop the faker and exit
    Stop,

    /// Request a scrape from the tracker
    Scrape,

    /// Get current stats (triggers immediate stats event)
    Stats,
}

impl InputCommand {
    /// Parse a JSON line into a command
    pub fn parse(line: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(line)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pause() {
        let cmd = InputCommand::parse(r#"{"command":"pause"}"#).unwrap();
        assert!(matches!(cmd, InputCommand::Pause));
    }

    #[test]
    fn test_parse_resume() {
        let cmd = InputCommand::parse(r#"{"command":"resume"}"#).unwrap();
        assert!(matches!(cmd, InputCommand::Resume));
    }

    #[test]
    fn test_parse_stop() {
        let cmd = InputCommand::parse(r#"{"command":"stop"}"#).unwrap();
        assert!(matches!(cmd, InputCommand::Stop));
    }

    #[test]
    fn test_parse_scrape() {
        let cmd = InputCommand::parse(r#"{"command":"scrape"}"#).unwrap();
        assert!(matches!(cmd, InputCommand::Scrape));
    }

    #[test]
    fn test_parse_stats() {
        let cmd = InputCommand::parse(r#"{"command":"stats"}"#).unwrap();
        assert!(matches!(cmd, InputCommand::Stats));
    }
}
