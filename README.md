# `SRC-DST-Clarifier`

![](https://img.shields.io/crates/v/src-dst-clarifier)
![](https://img.shields.io/crates/d/src-dst-clarifier)
![](https://img.shields.io/crates/l/src-dst-clarifier)
![](https://img.shields.io/docsrs/src-dst-clarifier)
![](https://img.shields.io/github/stars/eternal-io/src-dst-clarifier?style=social)

***(WIP)***

Give SRC and DST path, each may be a FILE or a DIR (even STDIO). Handle situations well and produce iterator over FILE-FILE pairs.

``` rust
SrcDstConfig::new("png").parse("input.jpg", None);
// [./input.jpg => ./A01123-0456-0789.png]

SrcDstConfig::new("png").parse("input.jpg", Some("output.jpg"));
// [./input.jpg => ./output.jpg]

SrcDstConfig::new("png").parse("./frames", None);
// [./frames/0001.jpg   => ./A01123-0456-0789/0001.jpg]
// [./frames/0002.jpg   => ./A01123-0456-0789/0002.jpg]
// [./frames/0003.jpg   => ./A01123-0456-0789/0003.jpg]
//  ...
// [./frames/xxxx.jpg   => ./A01123-0456-0789/xxxx.jpg]

SrcDstConfig::new("png").parse("-", Some("-"));
// [<io::Stdin> => <io::Stdout>]
```

See documentation on [docs.rs](https://docs.rs/src-dst-clarifier).

## TODOs

- Add `wildcard matcher` and `(number) range filter` to SRC.

    ``` shell
    -i "./*.jpg"
    -i "./4???.jpg"
    -i "./{:04d}.jpg"
    -i "./{1..=999:04d}.jpg"
    ```
