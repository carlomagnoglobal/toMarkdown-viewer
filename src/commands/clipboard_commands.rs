use crate::clipboard_ops;
use std::path::Path;
use std::io::Write;

#[tauri::command]
pub async fn copy_file(
    path: String,
    format: String,
    file_type: String,
) -> Result<String, String> {
    let p = Path::new(&path);

    let content = match format.as_str() {
        "base64" => clipboard_ops::copy_as_base64(p)?,
        "markdown" => clipboard_ops::copy_as_markdown(p, &file_type)?,
        "hex" => clipboard_ops::copy_as_hex(p)?,
        "sha256" => clipboard_ops::copy_sha256(p)?,
        "md5" => clipboard_ops::copy_md5(p)?,
        "crc" => clipboard_ops::copy_crc(p)?,
        _ => return Err(format!("Unknown format: {}", format)),
    };

    // Copy to system clipboard
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        Command::new("pbcopy")
            .stdin(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| e.to_string())?
            .stdin
            .ok_or("Failed to open pbcopy stdin")?
            .write_all(content.as_bytes())
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        Command::new("cmd")
            .args(&["/C", "clip"])
            .stdin(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| e.to_string())?
            .stdin
            .ok_or("Failed to open clip stdin")?
            .write_all(content.as_bytes())
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "linux")]
    {
        use std::process::Command;
        Command::new("xclip")
            .args(&["-selection", "clipboard"])
            .stdin(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| e.to_string())?
            .stdin
            .ok_or("Failed to open xclip stdin")?
            .write_all(content.as_bytes())
            .map_err(|e| e.to_string())?;
    }

    Ok(content)
}
