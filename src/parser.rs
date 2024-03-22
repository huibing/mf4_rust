pub mod parser {
    use mf4_parse::block::*;
    use rust_embed::RustEmbed;
    use std::io::{BufReader, Seek, Read, SeekFrom};
    use std::path::PathBuf;
    use std::fs::File;


    #[derive(RustEmbed)]
    #[folder = "config/"]
    #[prefix = "config/"]
    struct Asset;   // compile config file asset to binary

    pub struct MdfInfo {
        version: String,
    }

    pub fn parse_toml(block_name: &str) -> Result<BlockDesc, Box<dyn std::error::Error>> {
        let mut path = PathBuf::from("config/");
        path.push(block_name);
        path.set_extension("toml");
        let toml_file = Asset::get(path.to_str().ok_or("")?).ok_or("")?;
        Ok(toml::from_str(std::str::from_utf8(toml_file.data.as_ref())?)?)
    }

    pub fn parse_mdf_id_block(file: &mut BufReader<File>, mdf: &mut MdfInfo) -> Result<(), Box<dyn std::error::Error>>{
        // manually parse id block
        file.seek(std::io::SeekFrom::Start(0))?;
        let mut buf = [0u8;8];

        file.read_exact(&mut buf)?;

        Ok(())
    }
    

}


#[cfg(test)]
pub mod parser_test {
    use mf4_parse::block::*;
    use crate::parser::*;

    #[test]
    fn test_parse_toml() {
        let block = parser::parse_toml("dg").unwrap();
        assert!(block.check_id("##DG".as_bytes()));
    }
}