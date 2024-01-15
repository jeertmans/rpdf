pub mod traits;

mod annotations;

use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use is_terminal::IsTerminal;
use termcolor::{ColorChoice, StandardStream};

use traits::Execute;

#[derive(Debug, Parser)]
#[command(
    author,
    version,
    about,
    propagate_version(true),
    subcommand_required(true),
    verbatim_doc_comment
)]
pub struct Cli {
    /// Specify WHEN to colorize output.
    #[arg(short, long, value_name = "WHEN", default_value = "auto", default_missing_value = "always", num_args(0..=1), require_equals(true))]
    pub color: clap::ColorChoice,
    /// Subcommand.
    #[command(subcommand)]
    #[allow(missing_docs)]
    pub command: Command,
    #[command(flatten)]
    pub verbose: clap_verbosity_flag::Verbosity,
}

/// Enumerate all possible commands.
#[derive(Debug, Subcommand)]
#[allow(missing_docs)]
pub enum Command {
    #[clap(visible_alias = "ann")]
    Annotations(annotations::AnnotationsCommand),
    Completions(complete::CompleteCommand),
}

impl Cli {
    /// Return a standard output stream that optionally supports color.
    #[must_use]
    fn stdout(&self) -> StandardStream {
        let mut choice: ColorChoice = match self.color {
            clap::ColorChoice::Auto => ColorChoice::Auto,
            clap::ColorChoice::Always => ColorChoice::Always,
            clap::ColorChoice::Never => ColorChoice::Never,
        };

        if choice == ColorChoice::Auto && !std::io::stdout().is_terminal() {
            choice = ColorChoice::Never;
        }

        StandardStream::stdout(choice)
    }

    /// Execute command, possibily returning an error.
    pub fn execute(self) -> Result<()> {
        let mut stdout = self.stdout();

        match self.command {
            Command::Annotations(cmd) => {
                cmd.execute(&mut stdout)?;
            },
            Command::Completions(cmd) => {
                cmd.execute(&mut stdout)?;
            },
        }
        Ok(())
    }
}

/// Build a command from the top-level command line structure.
#[must_use]
pub fn build_cli() -> clap::Command {
    Cli::command()
}

pub(crate) mod complete {
    //! Completion scripts generation with [`clap_complete`].

    use anyhow::Result;
    use clap::{Command, Parser};
    use clap_complete::{generate, shells::Shell};
    use std::io::Write;

    /// Command structure to generate complete scripts.
    #[derive(Debug, Parser)]
    #[command(
    about = "Generate tab-completion scripts for supported shells",
    after_help = "Use --help for installation help.",
    after_long_help = COMPLETIONS_HELP
)]
    pub struct CompleteCommand {
        /// Shell for which to completion script is generated.
        #[arg(value_enum, ignore_case = true)]
        shell: Shell,
    }

    impl CompleteCommand {
        /// Generate completion file for current shell and write to buffer.
        pub fn generate_completion_file<F, W>(&self, build_cli: F, buffer: &mut W)
        where
            F: FnOnce() -> Command,
            W: Write,
        {
            generate(self.shell, &mut build_cli(), "rpdf", buffer);
        }

        /// Execute command by writing completion script to stdout.
        pub fn execute<W>(&self, stdout: &mut W) -> Result<()>
        where
            W: Write,
        {
            self.generate_completion_file(super::build_cli, stdout);
            Ok(())
        }
    }

    pub(crate) static COMPLETIONS_HELP: &str = r"DISCUSSION:
    Enable tab completion for Bash, Fish, Zsh, or PowerShell
    Elvish shell completion is currently supported, but not documented below.
    The script is output on `stdout`, allowing one to re-direct the
    output to the file of their choosing. Where you place the file
    will depend on which shell, and which operating system you are
    using. Your particular configuration may also determine where
    these scripts need to be placed.
    Here are some common set ups for the three supported shells under
    Unix and similar operating systems (such as GNU/Linux).
    BASH:
    Completion files are commonly stored in `/etc/bash_completion.d/` for
    system-wide commands, but can be stored in
    `~/.local/share/bash-completion/completions` for user-specific commands.
    Run the command:
        $ mkdir -p ~/.local/share/bash-completion/completions
        $ rpdf completions bash >> ~/.local/share/bash-completion/completions/rpdf
    This installs the completion script. You may have to log out and
    log back in to your shell session for the changes to take effect.
    BASH (macOS/Homebrew):
    Homebrew stores bash completion files within the Homebrew directory.
    With the `bash-completion` brew formula installed, run the command:
        $ mkdir -p $(brew --prefix)/etc/bash_completion.d
        $ rpdf completions bash > $(brew --prefix)/etc/bash_completion.d/rpdf.bash-completion
    FISH:
    Fish completion files are commonly stored in
    `$HOME/.config/fish/completions`. Run the command:
        $ mkdir -p ~/.config/fish/completions
        $ rpdf completions fish > ~/.config/fish/completions/rpdf.fish
    This installs the completion script. You may have to log out and
    log back in to your shell session for the changes to take effect.
    ZSH:
    ZSH completions are commonly stored in any directory listed in
    your `$fpath` variable. To use these completions, you must either
    add the generated script to one of those directories, or add your
    own to this list.
    Adding a custom directory is often the safest bet if you are
    unsure of which directory to use. First create the directory; for
    this example we'll create a hidden directory inside our `$HOME`
    directory:
        $ mkdir ~/.zfunc
    Then add the following lines to your `.zshrc` just before
    `compinit`:
        fpath+=~/.zfunc
    Now you can install the completions script using the following
    command:
        $ rpdf completions zsh > ~/.zfunc/_rpdf
    You must then either log out and log back in, or simply run
        $ exec zsh
    for the new completions to take effect.
    CUSTOM LOCATIONS:
    Alternatively, you could save these files to the place of your
    choosing, such as a custom directory inside your $HOME. Doing so
    will require you to add the proper directives, such as `source`ing
    inside your login script. Consult your shells documentation for
    how to add such directives.
    POWERSHELL:
    The powershell completion scripts require PowerShell v5.0+ (which
    comes with Windows 10, but can be downloaded separately for windows 7
    or 8.1).
    First, check if a profile has already been set
        PS C:\> Test-Path $profile
    If the above command returns `False` run the following
        PS C:\> New-Item -path $profile -type file -force
    Now open the file provided by `$profile` (if you used the
    `New-Item` command it will be
    `${env:USERPROFILE}\Documents\WindowsPowerShell\Microsoft.PowerShell_profile.ps1`
    Next, we either save the completions file into our profile, or
    into a separate file and source it inside our profile. To save the
    completions into our profile simply use
        PS C:\> rpdf completions powershell >> ${env:USERPROFILE}\Documents\WindowsPowerShell\Microsoft.PowerShell_profile.ps1
    SOURCE:
        This documentation is directly taken from: https://github.com/rust-lang/rustup/blob/8f6b53628ad996ad86f9c6225fa500cddf860905/src/cli/help.rs#L157";
}
