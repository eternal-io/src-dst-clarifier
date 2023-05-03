# src-dst-clarifier

***(WIP)***

Given SRC and DST path, handle situations well and produce `Iterator<(reader, writer)>`.

See documentation on [docs.rs](https://docs.rs/src-dst-clarifier).

## TODOs

- Add `wildcard matcher` and `(number) range filter` to SRC.

    ``` shell
    -i "./*.jpg"
    -i "./4???.jpg"
    -i "./{:04d}.jpg"
    -i "./{1..=999:04d}.jpg"
    ```

- IO multiple files from Stdio?
