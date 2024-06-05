use std::env;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

const STRUCTS: &str = stringify! {
    use uncased::UncasedStr;

    #[derive(Clone, Copy, Debug)]
    pub(crate) enum CommandKeywords {
        Ping,
        Echo,
        Command,
        Get,
        Set,
    }

    #[derive(Clone, Copy, Debug)]
    pub(crate) enum SetParams {
        EX,
        PX,
    }
};

fn main() {
    let path = Path::new(&env::var("OUT_DIR").unwrap()).join("commands.rs");
    let mut file = BufWriter::new(File::create(path).unwrap());

    writeln!(&mut file, "{}", STRUCTS).expect("Failed to write CommandKeywords to file");

    writeln!(
        &mut file,
        "pub(crate) static COMMAND_KEYWORDS: phf::Map<&'static uncased::UncasedStr, CommandKeywords> = \n{}",
        phf_codegen::Map::<&uncased::UncasedStr>::new()
            .entry("ping".into(), "CommandKeywords::Ping")
            .entry("echo".into(), "CommandKeywords::Echo")
            .entry("command".into(), "CommandKeywords::Command")
            .entry("get".into(), "CommandKeywords::Get")
            .entry("set".into(), "CommandKeywords::Set")
            .build()
    )
        .expect("Failed to write COMMAND_KEYWORDS to file");
    writeln!(&mut file, ";\n\n").expect("Failed to write new line to file");

    writeln!(
        &mut file,
        "pub(crate) static SET_PARAMS: phf::Map<&'static uncased::UncasedStr, SetParams> = \n{}",
        phf_codegen::Map::<&uncased::UncasedStr>::new()
            .entry("ex".into(), "SetParams::EX")
            .entry("px".into(), "SetParams::PX")
            .build()
    )
    .expect("Failed to write SET_PARAMS to file");
    writeln!(&mut file, ";").expect("Failed to write new line to file");
}
