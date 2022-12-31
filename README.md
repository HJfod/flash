# :zap: Flash

A simple tool for generating beautiful documentation for C++.

Built for projects that use CMake and host their docs on GitHub Pages.

:rocket: Decently fast (~20s to build docs for [Geode](https://github.com/geode-sdk/geode))

:rocket: Beautiful, easily legible output

:rocket: Opinionated with minimal configuration required (no 400-line Doxyfiles required)

## :question: Why?

Because I tried Doxygen for five seconds and found its output way too bloated and way too ugly.

The goals of this project is to generate documentation that looks beautiful and is super easy to navigate. I also wanted to make just browsing the docs intuitive and simple to encourage learning about what tools are available before you find an usecase for them.

## :point_right: Usage

Flash can be compiled using `cargo build` as usual for Rust projects.

Running Flash requires the following command line arguments: `flash -i <input_dir> -o <output_dir> [--overwrite]`

`input_dir` points to a directory with the project you want to generate docs for, and `output_dir` is where to place the generated documentation pages. Unless `--overwrite` is specified, `output_dir` must not exist prior to running Flash.

> :warning: `output_dir` should be a relative path, or bad things may happen with the links on the docs page.

> :warning: The output directory should be the same relative root path as where the docs will eventually live, so for example doing `-o docs` means that the docs root URL on the website should be `site.com/docs`.

Configuring Flash happens through a `flash.toml` file at the root of the project.

| Key                   | Required | Default  | Description |
| --------------------- | -------- | -------- | ----------- |
| `project.name`          | Yes      | None     | Project name
| `project.version`       | Yes      | None     | Project version
| `project.repository`    | No       | None     | GitHub repository
| `docs.include`          | Yes      | None     | Headers files to include for the documentation. Supports glob, so `**/*.hpp` will match all headers under project root and subdirectories. Note that any files included by the specified headers are considered when building docs aswell, so if you have one root header that includes all the project's headers, you should just point `docs.include` to that only |
| `docs.exclude`          | No       | None     | Header files captured by `docs.include` that should actually be excluded from documentation. This does not exclude files if they are included through other files in `docs.include` with `#include` |
| `docs.tree`             | No       | None     | The online tree base to use for documentation. Allows Flash to automatically generate links to the headers. Flash assumes that the input directory root is the same as the tree root; as in, a file that exist at `some/dir/header.hpp` in the input directory exist at `root/some/dir/header.hpp` |
| `run.prebuild`          | No       | None     | List of command line commands to run prior to configuring docs |
| `analysis.compile-args` | No | None | List of arguments to pass to LibClang |
| `cmake.config-args`     | No       | None     | List of arguments to pass to CMake when configuring |
| `cmake.build-args`      | No       | None     | List of arguments to pass to CMake when building, if `cmake.build` is true |
| `cmake.build`           | No       | `false`  | Whether to actually build the CMake project or not |
| `cmake.infer-args-from` | Yes (if `cmake` is specified) | None | What source file to get compilation arguments (include paths, defines, etc.) from |
| `template.class` | No | `templates/class.html` | The file to use as the base for formatting docs for classes |
| `template.struct-` (sic.) | No | `templates/struct.html` | The file to use as the base for formatting docs for structs |
| `template.function` | No | `templates/function.html` | The file to use as the base for formatting docs for functions |
| `template.file` | No | `templates/file.html` | The file to use as the base for formatting docs for files |
| `template.index` | No | `templates/index.html` | The file to use as the base for formatting the docs root page |
| `template.head` | No | `templates/head.html` | The file to use as the base for formatting the `<head>` element for each docs page |
| `template.nav` | No | `templates/nav.html` | The file to use as the base for formatting the navigation browser |
| `template.page` | No | `templates/page.html` | The file to use as the base for formatting a docs page |
| `scripts.css` | No | All the `css` files in `templates` | The CSS files to include with the docs. All the files are placed at root |
| `scripts.js` | No | All the `js` files in `templates` | The JS files to include with the docs. All the files are placed at root |
