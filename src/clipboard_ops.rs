use std::path::Path;
use sha2::{Sha256, Sha512, Digest};
use sha1::Sha1;
use md5;
use base64::Engine;
use crc32fast::Hasher;

pub fn copy_as_base64(file_path: &Path) -> Result<String, String> {
    let content = std::fs::read(file_path)
        .map_err(|e| format!("Failed to read file: {}", e))?;
    Ok(base64::engine::general_purpose::STANDARD.encode(&content))
}

pub fn copy_as_markdown(file_path: &Path, file_type: &str) -> Result<String, String> {
    let content = std::fs::read_to_string(file_path)
        .map_err(|e| format!("Failed to read file: {}", e))?;

    match file_type {
        "markdown" => Ok(content),
        "code" => {
            let lang = detect_language(file_path);
            Ok(format!("```{}\n{}\n```", lang, content))
        }
        "image" => {
            let b64 = copy_as_base64(file_path)?;
            Ok(format!("![image](data:image/{};base64,{})", "png", b64))
        }
        _ => {
            Ok(format!("```\n{}\n```", content))
        }
    }
}

pub fn copy_as_hex(file_path: &Path) -> Result<String, String> {
    let content = std::fs::read(file_path)
        .map_err(|e| format!("Failed to read file: {}", e))?;

    let hex: String = content
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect();
    Ok(hex)
}

pub fn copy_sha256(file_path: &Path) -> Result<String, String> {
    let content = std::fs::read(file_path)
        .map_err(|e| format!("Failed to read file: {}", e))?;

    let mut hasher = Sha256::new();
    hasher.update(&content);
    let result = hasher.finalize();

    Ok(format!("{:x}", result))
}

pub fn copy_md5(file_path: &Path) -> Result<String, String> {
    let content = std::fs::read(file_path)
        .map_err(|e| format!("Failed to read file: {}", e))?;

    let digest = md5::compute(&content);
    Ok(format!("{:x}", digest))
}

pub fn copy_crc(file_path: &Path) -> Result<String, String> {
    let content = std::fs::read(file_path)
        .map_err(|e| format!("Failed to read file: {}", e))?;

    let mut crc = 0u32;
    for byte in &content {
        crc = crc.wrapping_mul(31).wrapping_add(*byte as u32);
    }

    Ok(format!("{:08x}", crc))
}

pub fn copy_sha1(file_path: &Path) -> Result<String, String> {
    let content = std::fs::read(file_path)
        .map_err(|e| format!("Failed to read file: {}", e))?;

    let mut hasher = Sha1::new();
    hasher.update(&content);
    let result = hasher.finalize();

    Ok(format!("{:x}", result))
}

pub fn copy_sha512(file_path: &Path) -> Result<String, String> {
    let content = std::fs::read(file_path)
        .map_err(|e| format!("Failed to read file: {}", e))?;

    let mut hasher = Sha512::new();
    hasher.update(&content);
    let result = hasher.finalize();

    Ok(format!("{:x}", result))
}

pub fn copy_blake3(file_path: &Path) -> Result<String, String> {
    let content = std::fs::read(file_path)
        .map_err(|e| format!("Failed to read file: {}", e))?;

    let hash = blake3::hash(&content);
    let hex: String = hash.as_bytes()
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect();
    Ok(hex)
}

pub fn copy_crc32(file_path: &Path) -> Result<String, String> {
    let content = std::fs::read(file_path)
        .map_err(|e| format!("Failed to read file: {}", e))?;

    let mut hasher = Hasher::new();
    hasher.update(&content);
    let crc = hasher.finalize();

    Ok(format!("{:08x}", crc))
}

fn detect_language(path: &Path) -> String {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| match ext {
            "rs" => "rust",
            "py" => "python",
            "js" => "javascript",
            "ts" => "typescript",
            "go" => "go",
            "c" => "c",
            "cpp" => "cpp",
            "java" => "java",
            "rb" => "ruby",
            "php" => "php",
            _ => "",
        })
        .unwrap_or("")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[test]
    fn test_copy_as_base64() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "hello").unwrap();

        let b64 = copy_as_base64(&test_file).unwrap();
        assert_eq!(b64, "aGVsbG8=");
    }

    #[test]
    fn test_copy_sha256() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "hello").unwrap();

        let hash = copy_sha256(&test_file).unwrap();
        assert_eq!(hash.len(), 64); // SHA256 in hex is 64 chars
    }

    #[test]
    fn test_copy_as_markdown_code() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.rs");
        fs::write(&test_file, "fn main() {}").unwrap();

        let md = copy_as_markdown(&test_file, "code").unwrap();
        assert!(md.contains("```rust"));
        assert!(md.contains("fn main()"));
    }

    #[test]
    fn test_copy_sha1() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "hello").unwrap();

        let hash = copy_sha1(&test_file).unwrap();
        assert_eq!(hash.len(), 40); // SHA1 in hex is 40 chars
    }

    #[test]
    fn test_copy_sha512() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "hello").unwrap();

        let hash = copy_sha512(&test_file).unwrap();
        assert_eq!(hash.len(), 128); // SHA512 in hex is 128 chars
    }

    #[test]
    fn test_copy_blake3() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "hello").unwrap();

        let hash = copy_blake3(&test_file).unwrap();
        assert_eq!(hash.len(), 64); // BLAKE3 in hex is 64 chars
    }

    #[test]
    fn test_copy_crc32() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "hello").unwrap();

        let hash = copy_crc32(&test_file).unwrap();
        assert_eq!(hash.len(), 8); // CRC32 in hex is 8 chars (32 bits)
    }

    #[test]
    fn test_copy_md5() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "hello").unwrap();

        let hash = copy_md5(&test_file).unwrap();
        assert_eq!(hash.len(), 32); // MD5 in hex is 32 chars
    }
}
