extern crate phf_codegen;
use convert_case::{Case, Casing};
use std::env;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let out_dir = env::var("OUT_DIR").unwrap();
    let domain_map_path = Path::new(&out_dir).join("domain_map.rs");
    let url_trie_path = Path::new(&out_dir).join("url_ignore_trie.rs");
    let blockers_dir = Path::new(&out_dir).join("blockers");
    fs::create_dir_all(&blockers_dir).unwrap();

    let pattern_dir = "url_patterns/domains";

    generate_domain_map(&domain_map_path, pattern_dir);
    generate_url_ignore_tries(&url_trie_path, pattern_dir);
    generate_blockers(&blockers_dir, pattern_dir);
    generate_blockers_mod(&blockers_dir, pattern_dir);

    #[cfg(feature = "adblock_easylist")]
    easylist::fetch_lists(&out_dir);
}

fn generate_domain_map(domain_map_path: &Path, pattern_dir: &str) {
    let mut file = BufWriter::new(File::create(&domain_map_path).unwrap());
    let mut map = phf_codegen::Map::new();

    writeln!(file, "mod blockers;\nmod url_ignore_trie;").unwrap();
    writeln!(
        &mut file,
        "#[derive(Default, Debug, Clone, Copy, PartialEq)]"
    )
    .unwrap();
    writeln!(
        &mut file,
        r#"#[derive(serde::Serialize, serde::Deserialize)]"#
    )
    .unwrap();
    writeln!(&mut file, "pub enum NetworkInterceptManager {{").unwrap();

    let mut domain_variants = vec![];

    for entry in fs::read_dir(pattern_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();

        if let Some(domain_name) = path.file_stem().unwrap().to_str() {
            let enum_name = format_ident(domain_name).to_case(Case::UpperCamel);
            writeln!(&mut file, "    {},", enum_name).unwrap();
            domain_variants.push((domain_name.to_string(), enum_name.clone()));
            map.entry(
                format!("{}", domain_name),
                &format!("NetworkInterceptManager::{}", enum_name),
            );
        }
    }

    writeln!(&mut file, "    #[default]\n    Unknown,").unwrap(); // Default case
    writeln!(&mut file, "}}\n").unwrap();

    write!(
        file,
        "static DOMAIN_MAP: phf::Map<&'static str, NetworkInterceptManager> = {};\n",
        map.build()
    )
    .unwrap();

    writeln!(file, "impl NetworkInterceptManager {{").unwrap();
    writeln!(file, "    pub fn intercept_detection(&self, url: &str, ignore_visuals: bool, is_xhr: bool) -> bool {{").unwrap();
    writeln!(file, "        let mut should_block = false;").unwrap();
    writeln!(file, "        match self {{").unwrap();

    for (domain_name, enum_name) in domain_variants {
        let clean_name = domain_name.split('.').next().unwrap().to_lowercase();
        writeln!(
            file,
            "            NetworkInterceptManager::{} => {{",
            enum_name
        )
        .unwrap();
        writeln!(file, "                if is_xhr {{").unwrap();
        writeln!(
            file,
            "                    should_block = blockers::{}_blockers::block_xhr(url);",
            clean_name
        )
        .unwrap();
        writeln!(file, "                }} else {{").unwrap();
        writeln!(
            file,
            "                    should_block = blockers::{}_blockers::block_scripts(url);",
            clean_name
        )
        .unwrap();
        writeln!(
            file,
            "                    if !should_block && ignore_visuals {{"
        )
        .unwrap();
        writeln!(
            file,
            "                        should_block = blockers::{}_blockers::block_styles(url);",
            clean_name
        )
        .unwrap();
        writeln!(file, "                    }}").unwrap();
        writeln!(file, "                }}").unwrap();
        writeln!(file, "            }},").unwrap();
    }

    writeln!(file, "            NetworkInterceptManager::Unknown => (),").unwrap();

    writeln!(file, "        }}").unwrap();
    writeln!(file, "        should_block").unwrap();
    writeln!(file, "    }}").unwrap();
    writeln!(file, "}}").unwrap();
}

fn generate_url_ignore_tries(url_trie_path: &Path, pattern_dir: &str) {
    let mut file = BufWriter::new(File::create(url_trie_path).unwrap());

    writeln!(file, "use crate::trie::Trie;").unwrap();
    writeln!(file, "use std::sync::LazyLock;").unwrap();

    for category in &["scripts", "xhr", "styles"] {
        if let Ok(domain_entries) = fs::read_dir(pattern_dir) {
            for domain_entry in domain_entries {
                let domain_entry = domain_entry.unwrap();
                let domain_path = domain_entry.path();

                if domain_path.is_dir() {
                    let domain_name = domain_path.file_name().unwrap().to_str().unwrap();
                    let category_domain_path = domain_path.join(category);

                    if let Ok(category_entries) = fs::read_dir(&category_domain_path) {
                        let trie_name = format_ident(&format!("{}_{}", domain_name, category));
                        writeln!(
                            file,
                            "pub static {}_TRIE: LazyLock<Trie> = LazyLock::new(|| {{",
                            trie_name.to_uppercase()
                        )
                        .unwrap();

                        let mut has_ignore = false;

                        for entry in category_entries {
                            let entry = entry.unwrap();
                            let path = entry.path();

                            if path.is_file() {
                                let contents = fs::read_to_string(path).unwrap();

                                if !has_ignore && !contents.is_empty() {
                                    writeln!(file, "let mut trie = Trie::new();").unwrap();
                                    has_ignore = true;
                                }

                                for pattern in contents.lines() {
                                    writeln!(file, "trie.insert({:?});", pattern.trim()).unwrap();
                                }
                            }
                        }

                        if !has_ignore {
                            writeln!(file, "let trie = Trie::new();").unwrap();
                        }

                        writeln!(file, "trie").unwrap();
                        writeln!(file, "}});").unwrap();
                    }
                }
            }
        }
    }
}

fn generate_blockers(blockers_dir: &Path, pattern_dir: &str) {
    if let Ok(domain_entries) = fs::read_dir(pattern_dir) {
        for domain_entry in domain_entries {
            let domain_entry = domain_entry.unwrap();
            let domain_path = domain_entry.path();

            if domain_path.is_dir() {
                let domain_name = domain_path.file_name().unwrap().to_str().unwrap();
                let file_name = format!("{}_blockers.rs", domain_name.split('.').next().unwrap());
                let file_path = blockers_dir.join(file_name);
                let mut file = BufWriter::new(File::create(file_path).unwrap());

                // Generate block_scripts
                let scripts_trie_name = format_ident(&format!("{}_scripts", domain_name));
                writeln!(file, "pub fn block_scripts(url: &str) -> bool {{").unwrap();
                writeln!(
                    file,
                    "    crate::intercept_manager::url_ignore_trie::{}_TRIE.contains_prefix(url)",
                    scripts_trie_name.to_uppercase()
                )
                .unwrap();
                writeln!(file, "}}\n").unwrap();

                // Generate block_styles
                let styles_trie_name = format_ident(&format!("{}_styles", domain_name));
                writeln!(file, "pub fn block_styles(url: &str) -> bool {{").unwrap();
                writeln!(
                    file,
                    "    crate::intercept_manager::url_ignore_trie::{}_TRIE.contains_prefix(url)",
                    styles_trie_name.to_uppercase()
                )
                .unwrap();
                writeln!(file, "}}\n").unwrap();

                // Generate block_xhr
                let xhr_trie_name = format_ident(&format!("{}_xhr", domain_name));
                writeln!(file, "pub fn block_xhr(url: &str) -> bool {{").unwrap();
                writeln!(
                    file,
                    "    crate::intercept_manager::url_ignore_trie::{}_TRIE.contains_prefix(url)",
                    xhr_trie_name.to_uppercase()
                )
                .unwrap();
                writeln!(file, "}}\n").unwrap();
            }
        }
    }
}

fn generate_blockers_mod(blockers_dir: &Path, pattern_dir: &str) {
    let mod_file_path = blockers_dir.join("mod.rs");
    let mut mod_file = BufWriter::new(File::create(mod_file_path).unwrap());

    if let Ok(domain_entries) = fs::read_dir(pattern_dir) {
        for domain_entry in domain_entries {
            let domain_entry = domain_entry.unwrap();
            let clean_name = domain_entry
                .file_name()
                .to_str()
                .unwrap_or_default()
                .split('.')
                .next()
                .unwrap()
                .to_lowercase();

            writeln!(mod_file, "pub mod {}_blockers;", clean_name).unwrap();
        }
    }
}

/// indents uppercased
fn format_ident(name: &str) -> String {
    name.replace('.', "_").replace('-', "_").to_uppercase()
}

#[cfg(feature = "adblock_easylist")]
mod easylist {
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::path::Path;

    const LISTS: &[(&str, &str, &str)] = &[
        ("easylist.to", "/easylist/easylist.txt", "easylist.txt"),
        ("easylist.to", "/easylist/easyprivacy.txt", "easyprivacy.txt"),
    ];

    pub fn fetch_lists(out_dir: &str) {
        for &(host, path, filename) in LISTS {
            let dest = Path::new(out_dir).join(filename);

            // Cache: skip if already downloaded and valid ABP content.
            if dest.exists() {
                if let Ok(content) = std::fs::read_to_string(&dest) {
                    if content.len() > 1024 && content.contains("[Adblock Plus") {
                        continue;
                    }
                }
            }

            match fetch_https(host, path) {
                Ok(body) => {
                    if body.contains("[Adblock Plus") && body.lines().count() > 100 {
                        let _ = std::fs::write(&dest, &body);
                        println!("cargo:warning=Downloaded {filename} ({} bytes)", body.len());
                    } else {
                        if !dest.exists() {
                            let _ = std::fs::write(&dest, "");
                        }
                        println!("cargo:warning={filename}: response failed validation, using fallback");
                    }
                }
                Err(e) => {
                    if !dest.exists() {
                        let _ = std::fs::write(&dest, "");
                    }
                    println!("cargo:warning=Failed to download {filename}: {e}");
                }
            }
        }
    }

    fn fetch_https(host: &str, path: &str) -> Result<String, Box<dyn std::error::Error>> {
        let connector = native_tls::TlsConnector::new()?;
        let stream = TcpStream::connect((host, 443))?;
        stream.set_read_timeout(Some(std::time::Duration::from_secs(30)))?;
        let mut tls = connector.connect(host, stream)?;

        let request = format!(
            "GET {path} HTTP/1.0\r\nHost: {host}\r\nAccept-Encoding: identity\r\n\r\n"
        );
        tls.write_all(request.as_bytes())?;

        let mut buf = Vec::with_capacity(4 * 1024 * 1024);
        tls.read_to_end(&mut buf)?;

        let response = String::from_utf8_lossy(&buf);

        // Follow a single 301/302 redirect.
        if response.starts_with("HTTP/1.0 301")
            || response.starts_with("HTTP/1.0 302")
            || response.starts_with("HTTP/1.1 301")
            || response.starts_with("HTTP/1.1 302")
        {
            if let Some(loc) = extract_header(&response, "Location") {
                if let Some((rhost, rpath)) = parse_https_url(loc) {
                    return fetch_https_direct(&rhost, &rpath);
                }
            }
        }

        match response.find("\r\n\r\n") {
            Some(idx) => Ok(response[idx + 4..].to_string()),
            None => Err("malformed HTTP response".into()),
        }
    }

    fn fetch_https_direct(host: &str, path: &str) -> Result<String, Box<dyn std::error::Error>> {
        let connector = native_tls::TlsConnector::new()?;
        let stream = TcpStream::connect((host, 443))?;
        stream.set_read_timeout(Some(std::time::Duration::from_secs(30)))?;
        let mut tls = connector.connect(host, stream)?;

        let request = format!(
            "GET {path} HTTP/1.0\r\nHost: {host}\r\nAccept-Encoding: identity\r\n\r\n"
        );
        tls.write_all(request.as_bytes())?;

        let mut buf = Vec::with_capacity(4 * 1024 * 1024);
        tls.read_to_end(&mut buf)?;

        let response = String::from_utf8_lossy(&buf);
        match response.find("\r\n\r\n") {
            Some(idx) => Ok(response[idx + 4..].to_string()),
            None => Err("malformed HTTP response".into()),
        }
    }

    fn extract_header<'a>(response: &'a str, name: &str) -> Option<&'a str> {
        let header_end = response.find("\r\n\r\n").unwrap_or(response.len());
        let headers_section = &response[..header_end];
        let prefix = format!("{}: ", name.to_ascii_lowercase());

        for line in headers_section.split("\r\n") {
            if line.to_ascii_lowercase().starts_with(&prefix) {
                return Some(line[prefix.len()..].trim());
            }
        }
        None
    }

    fn parse_https_url(url: &str) -> Option<(String, String)> {
        let rest = url.strip_prefix("https://")?;
        let (host, path) = match rest.find('/') {
            Some(i) => (rest[..i].to_string(), rest[i..].to_string()),
            None => (rest.to_string(), "/".to_string()),
        };
        Some((host, path))
    }
}
