# Flash

A simple tool for generating beautiful documentation for C++.

Built for projects that use CMake and host their docs on GitHub Pages.

## Why?

Because I tried Doxygen for five seconds and found its output way too bloated and way too ugly.

The goals of this project is to generate documentation that looks beautiful and is super easy to navigate. I also wanted to make just browsing the docs intuitive and simple to encourage learning about what tools are available before you find an usecase for them.

## Usage

Flash can be compiled using `cargo build` as usual for Rust projects.

Running Flash requires the following command line arguments: `flash -i <input_dir> -o <output_dir> [--overwrite]`

`input_dir` points to a directory with the project you want to generate docs for, and `output_dir` is where to place the generated documentation pages. Unless `--overwrite` is specified, `output_dir` must not exist prior to running Flash.

Configuring Flash happens through a `flash.toml` file at the root of the project.

| Key                   | Required | Default  | Description |
| --------------------- | -------- | -------- | ----------- |
| project.name          | Yes      | None     | Project name
| project.version       | Yes      | None     | Project version
| project.repository    | No       | None     | GitHub repository
| docs.include          | Yes      | None     | Headers files to include for the documentation. Supports glob, so `**/*.hpp` will match all headers under project root and subdirectories. Note that any files included by the specified headers are considered when building docs aswell, so if you have one root header that includes all the project's headers, you should just point `docs.include` to that only |
| docs.exclude          | No       | None     | Header files captured by `docs.include` that should actually be excluded from documentation. This does not exclude files if they are included through other files in `docs.include` with `#include` |
| docs.tree             | No       | None     | The online tree base to use for documentation. Allows Flash to automatically generate links to the headers |
| run.prebuild          | No       | None     | List of command line commands to run prior to configuring docs |
| cmake.config-args     | No       | None     | List of arguments to pass to CMake when configuring |
| cmake.build-args      | No       | None     | List of arguments to pass to CMake when building, if `cmake.build` is true |
| cmake.build           | No       | `false`  | Whether to actually build the CMake project or not |
| cmake.infer-args-from | Yes (if `cmake` is specified) | None | What source file to get compilation arguments (include paths, defines, etc.) from |
