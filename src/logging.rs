//! Structured logging infrastructure for toMarkdown Viewer
//!
//! Provides initialization of tracing subscriber with:
//! - Console output (stdout)
//! - Daily rotating file logs in `.tomarkdown/logs`
//! - Timestamps and log levels (INFO, DEBUG, ERROR, etc.)

use std::path::Path;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

/// Initialize logging infrastructure with console and file output.
///
/// Creates `.tomarkdown/logs` directory if it doesn't exist,
/// sets up a daily rotating file appender, and initializes the tracing subscriber
/// with both console and file layers.
///
/// # Arguments
/// * `vault_root` - Path to the vault directory (logs will be in `.tomarkdown/logs` subdirectory)
///
/// # Returns
/// `Ok(())` on success, or error if initialization fails
///
/// # Example
/// ```ignore
/// let vault_path = std::path::Path::new("/path/to/vault");
/// init_logging(vault_path)?;
/// ```
pub fn init_logging(vault_root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // Create .tomarkdown/logs directory
    let logs_dir = vault_root.join(".tomarkdown").join("logs");
    std::fs::create_dir_all(&logs_dir)?;

    // Create daily rotating file appender
    let file_appender = tracing_appender::rolling::daily(&logs_dir, "vault.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    // Create a formatting layer for files with timestamps and levels
    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_target(true)
        .with_thread_ids(false)
        .with_file(true)
        .with_line_number(true);

    // Create a formatting layer for console with color
    let console_layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stdout)
        .with_ansi(true)
        .with_target(true)
        .with_thread_ids(false);

    // Set up the subscriber with environment-based filter
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(console_layer)
        .with(file_layer)
        .try_init()?;

    Ok(())
}
