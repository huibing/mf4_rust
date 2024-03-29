# rust_mf4
Work in progress!!

rust 语言编写的mf4读取库
只支持asam mf4 version >= 4.0
标准来源 <a href="https://www.asam.net/standards/detail/mdf/wiki/">ASAM MDF</a>

## 功能

- 读取mf4文件
- 读取mf4文件的header信息
- 读取mf4文件中的变量标签及信息
- 读取mf4文件中的数据
- 读取mf4文件中的

## 安装

```shell
    cargo install rust_mf4
```

or

```toml
    [dependencies]
    rust_mf4 = "0.1"
```

## 使用

```rust
    use rust_mf4::Mf4;
    let mf4 = Mf4::new("./test.mf4");
    let header = mf4.read_header().unwrap();
    println!("{:?}", header);
    let tags = mf4.read_tags().unwrap();
    println!("{:?}", tags);
    let data: VarData = mf4.read_data("measure_var").unwrap();
    println!("{:?}", data);
```
