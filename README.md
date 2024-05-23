# rust_mf4
A simple mf4 file reader by rust. 

Stardard reference:  <a href="https://www.asam.net/standards/detail/mdf/wiki/">ASAM MDF</a>;
Also you can access some of the demo mf4 files from <a href="https://www.asam.net/standards/detail/mdf/">here</a>.

## Features

- mf4 file reader
- read header info
- read channel info
- read float data from mf4 file
- read text data from mf4 file
- read array data from mf4 file
- read composed data from mf4 file
- read mf4 file with compressed data blocks

## Un-supported features

- Invalid bit flag processing 
- Bitfield text table conversion
- Inverse conversion
- CG and DG-template CA block
- Sample reduction block
- Compression method: Transposition + Deflate
- LD/FH/CH/AT blocks
  
Most of the above features are not supported because it is hard to obtain mf4 files with these features, so it's hard to develop and test these features.
In other words, it is rare that above features are utlized by tools that generate mf4 files.


## Install

Currently, this lib is not registered to crates.io.
You can clone this repo and use it locally.

```toml
## Cargo.toml
    [dependencies]
    mf4_parse = {path = "/local/path/to/rust_mf4_repo"}
```

Alternatively, you can specify this repo as a git dependency:

```toml
## Cargo.toml
    [dependencies]
    mf4_parse = {git = "https://github.com/huibing/rust_mf4.git"}
```


## Examples
Here is a simple example without proper error handling.

```rust
use mf4_parse::Mf4Wrapper;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut mf4 = Mf4Wrapper::new(PathBuf::from("./test_data/test.mf4"))?;
    println!("Header time stamp: {:?}", mf4.get_time_stamp());
    for (index, ch_name) in mf4.get_channel_names().iter().enumerate() {
        println!("{}th channel name: {:?}", index, ch_name);
    }
    println!("Channel1 data: {:?}", mf4.get_channel_data("Channel1").unwrap());
    println!("channel1's time stamp data: {:?}", mf4.get_channel_master_data("Channel1").unwrap());
    Ok(())
}
```

Also there are some other examples in the `src/main.rs` file.
