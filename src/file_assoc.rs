/*
Copyright ⓒ 2017 cargo-script contributors.

Licensed under the MIT license (see LICENSE or <http://opensource.org
/licenses/MIT>) or the Apache License, Version 2.0 (see LICENSE of
<http://www.apache.org/licenses/LICENSE-2.0>), at your option. All
files in the project carrying such notice may not be copied, modified,
or distributed except according to those terms.
*/
/*!
This module deals with setting up file associations.

Since this only makes sense on Windows, this entire module is Windows-only.
*/
#![cfg(windows)]

use crate::error::{Blame, Result};
use itertools::Itertools;
use std::io;

#[derive(Debug)]
pub enum Args {
    Install { amend_pathext: bool },
    Uninstall,
}

impl Args {
    pub fn subcommand() -> clap::App<'static, 'static> {
        use clap::{AppSettings, Arg, SubCommand};

        SubCommand::with_name("file-association")
            .about("Manage file assocations.")
            .setting(AppSettings::SubcommandRequiredElseHelp)
            .subcommand(SubCommand::with_name("install")
                .about("Install file associations.")
                .arg(Arg::with_name("amend_pathext")
                    .help("Add script extension to PATHEXT.  This allows scripts to be executed without typing the file extension.")
                    .long("amend-pathext")
                )
            )
            .subcommand(SubCommand::with_name("uninstall")
                .about("Uninstall file associations.")
            )
    }

    pub fn parse(m: &clap::ArgMatches<'_>) -> Self {
        match m.subcommand() {
            ("install", Some(m)) => Args::Install {
                amend_pathext: m.is_present("amend_pathext"),
            },
            ("uninstall", _) => Args::Uninstall,
            (name, _) => panic!("bad subcommand: {:?}", name),
        }
    }
}

pub fn try_main(args: Args) -> Result<i32> {
    match args {
        Args::Install { amend_pathext } => install(amend_pathext)?,
        Args::Uninstall => uninstall()?,
    }

    Ok(0)
}

fn install(amend_pathext: bool) -> Result<()> {
    use std::env;
    use winreg::enums as wre;
    use winreg::RegKey;

    // Set up file association.
    let cs_path = env::current_exe()?;
    let cs_path = cs_path.canonicalize()?;
    let rcs_path = cs_path.with_file_name("run-cargo-script.exe");

    if !rcs_path.exists() {
        return Err((Blame::Human, format!("{:?} not found", rcs_path)).into());
    }

    // We have to remove the `\\?\` prefix because, if we don't, the shell freaks out.
    let rcs_path = rcs_path.to_string_lossy();
    let rcs_path = if rcs_path.starts_with(r#"\\?\"#) {
        &rcs_path[4..]
    } else {
        &rcs_path[..]
    };

    let res = (|| -> io::Result<()> {
        let hlcr = RegKey::predef(wre::HKEY_CLASSES_ROOT);
        let (dot_crs, _) = hlcr.create_subkey(".crs")?;
        dot_crs.set_value("", &"CargoScript.Crs")?;

        let (cs_crs, _) = hlcr.create_subkey("CargoScript.Crs")?;
        cs_crs.set_value("", &"Cargo Script")?;

        let (sh_o_c, _) = cs_crs.create_subkey(r#"shell\open\command"#)?;
        sh_o_c.set_value("", &format!(r#""{}" "%1" %*"#, rcs_path))?;
        Ok(())
    })();

    match res {
        Ok(()) => (),
        Err(e) => {
            if e.kind() == io::ErrorKind::PermissionDenied {
                println!(
                    "Access denied.  Make sure you run this command from an administrator prompt."
                );
                return Err((Blame::Human, e).into());
            } else {
                return Err(e.into());
            }
        }
    }

    println!("Created run-cargo-script registry entry.");
    println!("- Handler set to: {}", rcs_path);

    // Amend PATHEXT.
    if amend_pathext {
        let hklm = RegKey::predef(wre::HKEY_LOCAL_MACHINE);
        let env =
            hklm.open_subkey(r#"SYSTEM\CurrentControlSet\Control\Session Manager\Environment"#)?;

        let pathext: String = env.get_value("PATHEXT")?;
        if !pathext.split(";").any(|e| e.eq_ignore_ascii_case(".crs")) {
            let pathext = pathext.split(";").chain(Some(".CRS")).join(";");
            env.set_value("PATHEXT", &pathext)?;
        }

        println!(
            "Added `.crs` to PATHEXT.  You may need to log out for the change to take effect."
        );
    }

    Ok(())
}

fn uninstall() -> Result<()> {
    use winreg::enums as wre;
    use winreg::RegKey;

    let mut ignored_missing = false;
    {
        let mut notify = || ignored_missing = true;

        let hlcr = RegKey::predef(wre::HKEY_CLASSES_ROOT);
        hlcr.delete_subkey(r#"CargoScript.Crs\shell\open\command"#)
            .ignore_missing_and(&mut notify)?;
        hlcr.delete_subkey(r#"CargoScript.Crs\shell\open"#)
            .ignore_missing_and(&mut notify)?;
        hlcr.delete_subkey(r#"CargoScript.Crs\shell"#)
            .ignore_missing_and(&mut notify)?;
        hlcr.delete_subkey(r#"CargoScript.Crs"#)
            .ignore_missing_and(&mut notify)?;
    }

    if ignored_missing {
        println!("Ignored some missing registry entries.");
    }
    println!("Deleted run-cargo-script registry entry.");

    {
        let hklm = RegKey::predef(wre::HKEY_LOCAL_MACHINE);
        let env =
            hklm.open_subkey(r#"SYSTEM\CurrentControlSet\Control\Session Manager\Environment"#)?;

        let pathext: String = env.get_value("PATHEXT")?;
        if pathext.split(";").any(|e| e.eq_ignore_ascii_case(".crs")) {
            let pathext = pathext
                .split(";")
                .filter(|e| !e.eq_ignore_ascii_case(".crs"))
                .join(";");
            env.set_value("PATHEXT", &pathext)?;
            println!("Removed `.crs` from PATHEXT.  You may need to log out for the change to take effect.");
        }
    }

    Ok(())
}

trait IgnoreMissing {
    fn ignore_missing_and<F>(self, f: F) -> Self
    where
        F: FnOnce();
}

impl IgnoreMissing for io::Result<()> {
    fn ignore_missing_and<F>(self, f: F) -> Self
    where
        F: FnOnce(),
    {
        match self {
            Ok(()) => Ok(()),
            Err(e) => {
                if e.kind() == io::ErrorKind::NotFound {
                    f();
                    Ok(())
                } else {
                    Err(e)
                }
            }
        }
    }
}
