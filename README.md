<h1 align="center">tapgen</h1>
<p align="center"><small><strong>Tony's Almighty Project Generator</strong></small></p>

A general-purpose, language-agnostic, yet simple and fast (i.e. *almighty*)
project generator to bootstrap and scaffold your projects easily.

Wanna start developing your new project real quick?
Run a single command and you can start coding right away.

Productivity++: No more wasting your time copy-and-pasting boilerplate starter code.

## Features

- Just run the binary: No intepreters or environment setup needed.

- Bloat-free command line tool: No more overwhelmed by tons of flags and options, i.e. it just simply works.

- Full power of the [`MiniJinja`](https://github.com/mitsuhiko/minijinja/) template engine.
  - fast and lightweight
  - familiar syntax of Jinja2
  - extensible filters and functions etc.

- Customizable prompts and template variables.
  - conditional prompts
  - range for integers
  - regex pattern validation for strings
  - single choice or multiple choices prompts

- Scripts as hooks that are run before and after generation.

- Like [Cookiecutter](https://github.com/cookiecutter/cookiecutter), but *faaaster* (written in Rust).

## Usage

```console
$ tapgen <SRC> [DST]
```

Currently, `SRC` can be one of the following:
- shorthand for git source<sup>[1](#git-source)</sup>:
  - `github:<owner>/<repo>`
  - `gitlab:<owner>/<repo>`
  - `bitbucket:<owner>/<repo>`

- shorthand for prefix source<sup>[2](#prefix-source)</sup>: `@:<path/to/template/under/prefix>`

- path to local `tapgen.toml` file or directory that contains a `tapgen.toml` file

<a name="git-source">1</a>: You can specify additional path in case when
the repository contains multiple templates, or when the template is several levels deep inside the repository,
e.g. `github:tnychn/templates/subdir1/subdir2`.

<a name="prefix-source">2</a>: Relative to the [prefix](#config) path, e.g. if the prefix is `/Users/tony/.tapgen`,
then `@:foo/bar` becomes `/Users/tony/.tapgen/foo/bar`.

## Config

```toml
# ~/.tapgen.config.toml

prefix = "<home>/.tapgen" # default; required
```

- `prefix`: path to directory; destination of git cloning and base path of prefix source.

## Definition

A `tapgen.toml` is a definition file that describes a template and its variables.

### Metadata

```toml
__name__ = "Hello World Template" # required
__author__ = "Tony Chan" # required
__url__ = "https://github.com/tnychn/hello-world-template"
__description__ = "A template."

__base__ = "./{{ name }}"
__copy__ = ["*.txt"]
__exclude__ = ["*.png"]
```

### Variables

TODO

---

<p align="center">
  <sub><strong>~ crafted with ♥︎ by tnychn ~</strong></sub>
  <br>
  <sub><strong>MIT © 2023 Tony Chan</strong></sub>
</p>
