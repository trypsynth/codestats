use std::{env::var, fs, path::Path};

fn main() {
    let json_path = Path::new("languages.json");
    let out_path = Path::new(&var("OUT_DIR").unwrap()).join("languages.rs");
    let json_data = fs::read_to_string(json_path).expect("Failed to read languages.json");
    let parsed: serde_json::Value = serde_json::from_str(&json_data).expect("Invalid JSON");
    let mut output = String::from("pub struct Language {\n");
    output.push_str("    pub name: &'static str,\n");
    output.push_str("    pub file_patterns: &'static [&'static str],\n");
    output.push_str("    pub line_comment: Option<&'static str>,\n");
    output.push_str("    pub block_comment: Option<(&'static str, &'static str)>,\n");
    output.push_str("}\n\n");
    output.push_str("pub static LANGUAGES: &[Language] = &[\n");
    for lang in parsed.as_array().expect("Expected array") {
        let name = lang["name"].as_str().unwrap();
        let patterns = lang["file_patterns"].as_array().unwrap();
        let line_comment = lang.get("line_comment").and_then(|v| v.as_str());
        let block_comment = lang.get("block_comment").and_then(|v| {
            let arr = v.as_array()?;
            Some((arr[0].as_str()?, arr[1].as_str()?))
        });
        output.push_str("    Language {\n");
        output.push_str(&format!("        name: \"{}\",\n", name));
        output.push_str("        file_patterns: &[");
        for p in patterns {
            output.push_str(&format!("\"{}\", ", p.as_str().unwrap()));
        }
        output.push_str("],\n");
        if let Some(lc) = line_comment {
            output.push_str(&format!("        line_comment: Some(\"{}\"),\n", lc));
        } else {
            output.push_str("        line_comment: None,\n");
        }
        if let Some((start, end)) = block_comment {
            output.push_str(&format!("        block_comment: Some((\"{}\", \"{}\")),\n", start, end));
        } else {
            output.push_str("        block_comment: None,\n");
        }
        output.push_str("    },\n");
    }
    output.push_str("];\n");
    fs::write(&out_path, output).expect("Failed to write generated languages.rs");
    println!("cargo:rerun-if-changed=languages.json");
}
