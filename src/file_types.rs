use std::path::Path;

/// File type classification for viewer routing
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileType {
    /// Markdown files (.md, .markdown)
    Markdown,
    /// Code files with detected language
    Code { language: String },
    /// Image files with format name
    Image { format: String },
    /// Unknown/binary files (fallback)
    Hex,
}

/// Language mappings from file extension to language name
const LANGUAGE_MAP: &[(&str, &str)] = &[
    // Web
    ("js", "javascript"),
    ("ts", "typescript"),
    ("jsx", "jsx"),
    ("tsx", "tsx"),
    ("html", "html"),
    ("htm", "html"),
    ("css", "css"),
    ("scss", "scss"),
    ("sass", "scss"),
    ("less", "less"),
    ("vue", "vue"),
    ("svelte", "svelte"),
    // Python
    ("py", "python"),
    ("pyx", "python"),
    ("pyi", "python"),
    ("ipynb", "python"),
    ("pyc", "python"),
    // Rust
    ("rs", "rust"),
    // Go
    ("go", "go"),
    ("mod", "go"),
    // C/C++
    ("c", "c"),
    ("cpp", "cpp"),
    ("cc", "cpp"),
    ("cxx", "cpp"),
    ("h", "c"),
    ("hpp", "cpp"),
    ("h++", "cpp"),
    // Java/JVM
    ("java", "java"),
    ("kt", "kotlin"),
    ("kts", "kotlin"),
    ("scala", "scala"),
    ("gradle", "groovy"),
    ("groovy", "groovy"),
    // C#/.NET
    ("cs", "csharp"),
    ("csx", "csharp"),
    ("fsx", "fsharp"),
    ("fs", "fsharp"),
    ("vb", "vbnet"),
    ("vbs", "vbnet"),
    // PHP
    ("php", "php"),
    ("phtml", "php"),
    ("php5", "php"),
    ("php7", "php"),
    ("php8", "php"),
    // Ruby
    ("rb", "ruby"),
    ("erb", "ruby"),
    ("rbx", "ruby"),
    ("gemfile", "ruby"),
    // Shell
    ("sh", "bash"),
    ("bash", "bash"),
    ("zsh", "bash"),
    ("fish", "fish"),
    ("ps1", "powershell"),
    // Lisp/Scheme
    ("el", "elisp"),
    ("lisp", "lisp"),
    ("cl", "lisp"),
    ("clj", "clojure"),
    ("cljs", "clojure"),
    // Config/Data/Markup
    ("json", "json"),
    ("jsonc", "json"),
    ("json5", "json"),
    ("yaml", "yaml"),
    ("yml", "yaml"),
    ("toml", "toml"),
    ("xml", "xml"),
    ("ini", "ini"),
    ("cfg", "ini"),
    ("conf", "bash"),
    ("env", "bash"),
    ("properties", "properties"),
    ("sql", "sql"),
    ("sqlite", "sql"),
    ("tsql", "sql"),
    ("plsql", "sql"),
    // Markup/Templates
    ("markdown", "markdown"),
    ("tex", "latex"),
    ("latex", "latex"),
    ("rst", "rst"),
    ("asciidoc", "asciidoc"),
    ("adoc", "asciidoc"),
    ("handlebars", "handlebars"),
    ("hbs", "handlebars"),
    // Build/CI
    ("dockerfile", "dockerfile"),
    ("dockerfile.prod", "dockerfile"),
    ("makefile", "makefile"),
    ("cmake", "cmake"),
    // Java build
    ("pom", "xml"),
    // Go modules
    ("sum", "bash"),
    // Perl
    ("pl", "perl"),
    ("pm", "perl"),
    // R/Statistics
    ("r", "r"),
    ("R", "r"),
    ("rmd", "markdown"),
    // Lua
    ("lua", "lua"),
    // Swift/iOS
    ("swift", "swift"),
    ("m", "objective-c"),
    ("mm", "objective-c++"),
    // Dart
    ("dart", "dart"),
    // Kotlin mobile
    ("gradle.kts", "kotlin"),
    // Text
    ("txt", "plaintext"),
];

/// Image format mappings from file extension to format name
const IMAGE_EXTENSIONS: &[(&str, &str)] = &[
    ("png", "png"),
    ("jpg", "jpeg"),
    ("jpeg", "jpeg"),
    ("gif", "gif"),
    ("svg", "svg"),
    ("webp", "webp"),
    ("avif", "avif"),
    ("tiff", "tiff"),
    ("tif", "tiff"),
];

/// Detect file type based on file extension
///
/// Detection order:
/// 1. Markdown (.md or .markdown) → Markdown
/// 2. Code files → Code { language }
/// 3. Image files → Image { format }
/// 4. Default → Hex
pub fn detect_file_type(path: &Path) -> FileType {
    // Get filename and convert to lowercase
    let filename = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_lowercase();

    // Check for Dockerfile (special case: no extension)
    if filename == "dockerfile" || filename.ends_with(".dockerfile") {
        return FileType::Code {
            language: "dockerfile".to_string(),
        };
    }

    // Get file extension and convert to lowercase
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    // Check if markdown
    if extension == "md" || extension == "markdown" {
        return FileType::Markdown;
    }

    // Check code extensions
    for (ext, language) in LANGUAGE_MAP {
        if extension == *ext {
            return FileType::Code {
                language: language.to_string(),
            };
        }
    }

    // Check image extensions
    for (ext, format) in IMAGE_EXTENSIONS {
        if extension == *ext {
            return FileType::Image {
                format: format.to_string(),
            };
        }
    }

    // Default to hex for unknown types
    FileType::Hex
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_markdown_extensions() {
        assert_eq!(detect_file_type(Path::new("file.md")), FileType::Markdown);
        assert_eq!(
            detect_file_type(Path::new("file.markdown")),
            FileType::Markdown
        );
    }

    #[test]
    fn test_case_insensitive() {
        assert_eq!(detect_file_type(Path::new("file.MD")), FileType::Markdown);
        assert_eq!(
            detect_file_type(Path::new("file.Markdown")),
            FileType::Markdown
        );
        match detect_file_type(Path::new("file.PY")) {
            FileType::Code { language } => assert_eq!(language, "python"),
            _ => panic!("Expected python code"),
        }
    }

    #[test]
    fn test_various_code_extensions() {
        match detect_file_type(Path::new("file.rs")) {
            FileType::Code { language } => assert_eq!(language, "rust"),
            _ => panic!("Expected rust code"),
        }

        match detect_file_type(Path::new("file.py")) {
            FileType::Code { language } => assert_eq!(language, "python"),
            _ => panic!("Expected python code"),
        }

        match detect_file_type(Path::new("file.ts")) {
            FileType::Code { language } => assert_eq!(language, "typescript"),
            _ => panic!("Expected typescript code"),
        }

        match detect_file_type(Path::new("file.go")) {
            FileType::Code { language } => assert_eq!(language, "go"),
            _ => panic!("Expected go code"),
        }
    }

    #[test]
    fn test_various_image_extensions() {
        match detect_file_type(Path::new("image.png")) {
            FileType::Image { format } => assert_eq!(format, "png"),
            _ => panic!("Expected png image"),
        }

        match detect_file_type(Path::new("image.jpg")) {
            FileType::Image { format } => assert_eq!(format, "jpeg"),
            _ => panic!("Expected jpeg image"),
        }

        match detect_file_type(Path::new("image.gif")) {
            FileType::Image { format } => assert_eq!(format, "gif"),
            _ => panic!("Expected gif image"),
        }

        match detect_file_type(Path::new("image.webp")) {
            FileType::Image { format } => assert_eq!(format, "webp"),
            _ => panic!("Expected webp image"),
        }
    }

    #[test]
    fn test_hex_fallback() {
        assert_eq!(detect_file_type(Path::new("file.bin")), FileType::Hex);
        assert_eq!(detect_file_type(Path::new("file.xyz")), FileType::Hex);
        assert_eq!(detect_file_type(Path::new("file.unknown")), FileType::Hex);
        assert_eq!(detect_file_type(Path::new("file")), FileType::Hex); // No extension
    }

    #[test]
    fn test_language_mapping_coverage() {
        // Test a sample from each language category
        match detect_file_type(Path::new("file.js")) {
            FileType::Code { language } => assert_eq!(language, "javascript"),
            _ => panic!("Expected javascript"),
        }

        match detect_file_type(Path::new("file.php")) {
            FileType::Code { language } => assert_eq!(language, "php"),
            _ => panic!("Expected php"),
        }

        match detect_file_type(Path::new("file.rb")) {
            FileType::Code { language } => assert_eq!(language, "ruby"),
            _ => panic!("Expected ruby"),
        }

        match detect_file_type(Path::new("file.json")) {
            FileType::Code { language } => assert_eq!(language, "json"),
            _ => panic!("Expected json"),
        }
    }
}
