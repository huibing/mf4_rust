use mf4_parse::block::BlockDesc;
use std::fs::File;
use std::io::BufReader;


fn main() -> Result<(), Box<dyn std::error::Error>> {
    // test string
    let toml_content = r####"  
        id = "##DG"
        implemented = true
        [link]
        dg_dg_next = ["DG"]
        dg_cg_first = ["CG"]
        dg_data = ["DT", "DV", "DZ", "DL", "LD", "HL"]
        dg_md_comment = ["TX", "MD"]
        [data]
        dg_rec_id_size = {data_type="BYTE", size=1}
        dg_reserved = {data_type="BYTE", size=7}
        "####;
    let block: BlockDesc = toml::from_str(toml_content)?;
    println!("{:?}", block);
    println!("{:?}", block.get_data_field("dg_rec_id_size").unwrap().get_data_type());
    println!("{:?}", block.get_link_block_type("dg_data").unwrap());

    assert!(block.check_id(b"##DG"));
    assert!(block.is_implemented());

    let file = File::open("./test/1.mf4")?;
    let mut buf = BufReader::new(file);   // offset 992   0x3e0
    let offset = 0x8db0;
    println!("{:?}", block.try_parse_buf(&mut buf, offset).unwrap());
    Ok(())
}