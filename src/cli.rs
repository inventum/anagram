//! CLI arguments.

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "anagram", about = "Terminal anagram puzzle")]
pub struct Cli {
    /// Path to config TOML (default: ~/.config/anagram/config.toml)
    #[arg(long, global = true)]
    pub config: Option<PathBuf>,

    /// Plain debug mode (no TUI) — reserved for future use
    #[arg(long, global = true)]
    pub plain: bool,

    /// Start a puzzle for this word (skips menu); default (play) mode only
    #[arg(long, value_name = "WORD")]
    pub word: Option<String>,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Interactive puzzle (default when this subcommand is omitted)
    Play {
        /// Start a puzzle for this word (skips menu)
        #[arg(long, value_name = "WORD")]
        word: Option<String>,
    },
    /// Resolve a scramble to lexicon words (TSV on stdout)
    Solve {
        /// Scrambled letters (e.g. eilnst)
        scramble: String,
    },
}

impl Cli {
    pub fn play_word(&self) -> Option<&str> {
        match &self.command {
            Some(Commands::Solve { .. }) => None,
            Some(Commands::Play { word }) => word.as_deref().or(self.word.as_deref()),
            None => self.word.as_deref(),
        }
    }

    pub fn solve_scramble(&self) -> Option<&str> {
        match &self.command {
            Some(Commands::Solve { scramble }) => Some(scramble.as_str()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_solve_subcommand() {
        let cli = Cli::try_parse_from(["anagram", "solve", "eilnst"]).unwrap();
        assert_eq!(cli.solve_scramble(), Some("eilnst"));
        assert!(cli.play_word().is_none());
    }

    #[test]
    fn parse_default_play_preserves_root_flags() {
        let cli = Cli::try_parse_from([
            "anagram",
            "--config",
            "/tmp/cfg.toml",
            "--plain",
            "--word",
            "plum",
        ])
        .unwrap();
        assert!(cli.command.is_none());
        assert_eq!(cli.config.as_deref(), Some(std::path::Path::new("/tmp/cfg.toml")));
        assert!(cli.plain);
        assert_eq!(cli.play_word(), Some("plum"));
    }

    #[test]
    fn parse_play_subcommand_with_word() {
        let cli = Cli::try_parse_from(["anagram", "play", "--word", "plum"]).unwrap();
        assert_eq!(cli.play_word(), Some("plum"));
        assert!(cli.solve_scramble().is_none());
    }
}