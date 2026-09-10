#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CliProvider {
    Claude,
    Codex,
    Gemini,
}

impl CliProvider {
    pub fn command_name(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
            Self::Gemini => "agy",
        }
    }

    pub fn executable_names(self) -> [&'static str; 3] {
        match self {
            Self::Claude => ["claude.exe", "claude.cmd", "claude"],
            Self::Codex => ["codex.exe", "codex.cmd", "codex"],
            Self::Gemini => ["agy.exe", "agy.cmd", "agy"],
        }
    }
}
