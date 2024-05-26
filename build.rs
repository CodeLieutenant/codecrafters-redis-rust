use std::env;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

fn main() {
    let path = Path::new(&env::var("OUT_DIR").unwrap()).join("commands.rs");
    let mut file = BufWriter::new(File::create(path).unwrap());

    let enum_str = r#"
        use uncased::UncasedStr;

        #[derive(Clone, Debug)]
        pub enum CommandKeywords {
            Ping,
            Echo,
        }"#;

    writeln!(&mut file, "{}", enum_str).expect("Failed to write CommandKeywords to file");

    writeln!(
        &mut file,
        "static COMMAND_KEYWORDS: phf::Map<&'static uncased::UncasedStr, CommandKeywords> = \n{}",
        phf_codegen::Map::<&uncased::UncasedStr>::new()
            .entry("ping".into(), "CommandKeywords::Ping")
            .entry("echo".into(), "CommandKeywords::Echo")
            .build()
    )
        .expect("Failed to write COMMAND_KEYWORDS to file");
    writeln!(&mut file, ";").unwrap();
}
