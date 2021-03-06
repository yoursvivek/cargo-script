/*
Copyright ⓒ 2015-2017 cargo-script contributors.

Licensed under the MIT license (see LICENSE or <http://opensource.org
/licenses/MIT>) or the Apache License, Version 2.0 (see LICENSE of
<http://www.apache.org/licenses/LICENSE-2.0>), at your option. All
files in the project carrying such notice may not be copied, modified,
or distributed except according to those terms.
*/
/*!
This module just contains any big string literals I don't want cluttering up the rest of the code.
*/

/**
The message output when the user invokes `cargo script` with no further arguments.  We need to do this ourselves because `clap` doesn't provide any way to generate this message manually.
*/
pub const NO_ARGS_MESSAGE: &'static str = "\
The following required arguments were not supplied:
\t'<script>'

USAGE:
\tcargo script [FLAGS OPTIONS] [--] <script> <args>...

For more information try --help";

/*
What follows are the templates used to wrap script input.
*/

/// Substitution for the script body.
pub const SCRIPT_BODY_SUB: &'static str = "script";

/// Substitution for the script prelude.
pub const SCRIPT_PRELUDE_SUB: &'static str = "prelude";

/// The template used for script file inputs.
pub const FILE_TEMPLATE: &'static str = r#"#{script}"#;

/// The template used for `--expr` input.
pub const EXPR_TEMPLATE: &'static str = r#"
#{prelude}
fn main() {
    let exit_code = match try_main() {
        Ok(()) => None,
        Err(e) => {
            use std::io::{self, Write};
            let _ = writeln!(io::stderr(), "Error: {}", e);
            Some(1)
        },
    };
    if let Some(exit_code) = exit_code {
        std::process::exit(exit_code);
    }
}

fn try_main() -> Result<(), Box<std::error::Error>> {
    match {#{script}} {
        __cargo_script_expr => println!("{:?}", __cargo_script_expr)
    }
    Ok(())
}
"#;

/*
Regarding the loop templates: what I *want* is for the result of the closure to be printed to standard output *only* if it's not `()`.

* TODO: Just use TypeId, dumbass.
* TODO: Merge the `LOOP_*` templates so there isn't duplicated code.  It's icky.
*/

/// The template used for `--loop` input, assuming no `--count` flag is also given.
pub const LOOP_TEMPLATE: &'static str = r#"
#{prelude}
use std::any::Any;
use std::io::prelude::*;

fn main() {
    let mut closure = enforce_closure(
{#{script}}
    );
    let mut line_buffer = String::new();
    let mut stdin = std::io::stdin();
    loop {
        line_buffer.clear();
        let read_res = stdin.read_line(&mut line_buffer).unwrap_or(0);
        if read_res == 0 { break }
        let output = closure(&line_buffer);

        let display = {
            let output_any: &Any = &output;
            !output_any.is::<()>()
        };

        if display {
            println!("{:?}", output);
        }
    }
}

fn enforce_closure<F, T>(closure: F) -> F
where F: FnMut(&str) -> T, T: 'static {
    closure
}
"#;

/// The template used for `--count --loop` input.
pub const LOOP_COUNT_TEMPLATE: &'static str = r#"
%p
use std::any::Any;
use std::io::prelude::*;

fn main() {
    let mut closure = enforce_closure(
{#{script}}
    );
    let mut line_buffer = String::new();
    let mut stdin = std::io::stdin();
    let mut count = 0;
    loop {
        line_buffer.clear();
        let read_res = stdin.read_line(&mut line_buffer).unwrap_or(0);
        if read_res == 0 { break }
        count += 1;
        let output = closure(&line_buffer, count);

        let display = {
            let output_any: &Any = &output;
            !output_any.is::<()>()
        };

        if display {
            println!("{:?}", output);
        }
    }
}

fn enforce_closure<F, T>(closure: F) -> F
where F: FnMut(&str, usize) -> T, T: 'static {
    closure
}
"#;

/// Substitution for the identifier-safe name of the script.
pub const MANI_NAME_SUB: &'static str = "name";

/// Substitution for the filesystem-safe name of the script.
pub const MANI_FILE_SUB: &'static str = "file";

/**
The default manifest used for packages.
*/
pub const DEFAULT_MANIFEST: &'static str = r##"
[package]
name = "#{name}"
version = "0.1.0"
authors = ["Anonymous"]

[[bin]]
name = "#{name}"
path = "#{file}.rs"
"##;

/**
The name of the package metadata file.
*/
pub const METADATA_FILE: &'static str = "metadata.json";

/**
Extensions to check when trying to find script input by name.
*/
pub const SEARCH_EXTS: &'static [&'static str] = &["crs", "rs"];

/**
When generating a package's unique ID, how many hex nibbles of the digest should be used *at most*?

The largest meaningful value is `40`.
*/
pub const ID_DIGEST_LEN_MAX: usize = 16;

/**
How old can stuff in the cache be before we automatically clear it out?

Measured in milliseconds.
*/
// It's been *one week* since you looked at me,
// cocked your head to the side and said "I'm angry."
pub const MAX_CACHE_AGE_MS: u64 = 1 * 7 * 24 * 60 * 60 * 1000;
