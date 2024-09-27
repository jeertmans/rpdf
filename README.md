# rpdf

> PDF command-line utils written in Rust.

*rpdf makes working with PDF annotations super easy!
It can `merge` annotations from multiple files,
some show statistics (`stats`) or `strip` specific (or all) annotations.*

[![Crates.io](https://img.shields.io/crates/v/rpdf)](https://crates.io/crates/rpdf)

1. [About](#about)
2. [CLI Reference](#cli-reference)
3. [Contributing](#contributing)

## About

rpdf is a Rust binary that aims to provides an open source and straighforward
command-line alternative to other tools such as
[PDF Annotator](https://www.pdfannotator.com/en/help/filescombine) and others.

*Disclaimer: rpdf is currently in an early stage, and does not implement many
features. It was first developed for my own use,
because I needed to merge annotations from PDFs I review with other people.
Do not hesitate to propose new features if you feel they could be intersting!*

## Installation

You can install the latest version with `cargo`.

```bash
> cargo install rpdf
```

## CLI Reference

The command line tool is pretty straighforward to use and is self-documented:

![CLI](https://user-images.githubusercontent.com/27275099/235343778-01eceb0a-e138-4dbc-be0c-824a4ae01f06.png)

Anytime you need help for a command, you can use `rpdf <COMMAND> --help`,
or `-h` for the short version.

### Examples

Below, you can find usage examples of `rpdf` in the terminal.

#### Annotations statistics

You can count how many annotations your file contains:

![statistics](https://user-images.githubusercontent.com/27275099/235343915-66d2206f-75d4-481a-9355-1be49aeedde6.png)

And you can also do this per page:

![statistics-per-page](https://user-images.githubusercontent.com/27275099/235344005-ab638e90-f619-4414-9b84-d23e25f7acf6.png)

#### Merge annotations

Say we have to files with the same content but different annotations:

![statistics-two-files](https://user-images.githubusercontent.com/27275099/235344066-2d06c7c6-a637-4ec6-b4ef-fde9e442afde.png)

You can merge the annotations from both files into one with the `merge` command,
and verify that all the annotations are present in the final product:

![merge](https://user-images.githubusercontent.com/27275099/235344220-d78a250b-35e1-47f8-919c-11e0dba4e62c.png)

#### Strip annotations

If you want to remove some annotations from a PDF,
you can do so with the `strip` command:

![strip](https://user-images.githubusercontent.com/27275099/235351437-5846c8bf-cd1c-4f27-9f3a-04257251251b.png)

By default, `strip` excludes `Link` annotations from the removal process.
You can modifiy the behavior with the `-e/--exclude` parameter.

## Contributing

Contributions are more than welcome! Please reach me via GitHub for any questions:
[Issues](https://github.com/jeertmans/rpdf/issues),
[Pull requests](https://github.com/jeertmans/rpdf/pulls) or
[Discussions](https://github.com/jeertmans/rpdf/discussions).
