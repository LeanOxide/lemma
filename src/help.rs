//! Help documentation for Lemma commands

pub static LEMMA_HELP: &str = r"DISCUSSION:
    lemma manages your installations of the Lean theorem prover.
    It places `lean` and `lake` binaries in your `PATH` that automatically
    select and, if necessary, download the Lean version described in your
    project's `lean-toolchain` file. You can also install, select, run,
    and uninstall Lean versions manually using the commands of the `lemma`
    executable.";

pub static SHOW_HELP: &str = r"DISCUSSION:
    Shows the name of the active toolchain and the version of `lean`.

    If there are multiple toolchains installed then all installed
    toolchains are listed as well.";

pub static DEFAULT_HELP: &str = r"DISCUSSION:
    Sets the default toolchain to the one specified.";

pub static TOOLCHAIN_HELP: &str = r"DISCUSSION:
    Many `lemma` commands deal with *toolchains*, a single
    installation of the Lean theorem prover. `lemma` supports multiple
    types of toolchains. The most basic track the official release
    channels: 'stable', 'beta', 'nightly', but `lemma` can also install toolchains from
    specific versions, direct URLs, and local builds.

    Standard release channel toolchain names have the following form:

        <channel> | <version> | <url>

        <channel>       = stable | beta | nightly
        <version>       = v4.x.x (e.g., v4.24.0)
        <url>           = https://... (direct download URL)

    'channel' is a named release channel (currently 'stable', 'beta', 'nightly' which track
    the latest releases in their respective channels). 'version' is an explicit version number,
    such as 'v4.24.0'. 'url' is a direct download URL to a Lean toolchain
    archive.

    lemma can also manage symlinked local toolchain builds, which are
    often used for developing Lean itself. For more information see
    `lemma toolchain help link`.";

pub static TOOLCHAIN_INSTALL_HELP: &str = r"DISCUSSION:
    Installs a specific Lean toolchain.

    The toolchain can be specified in several ways:
    - 'stable' for the latest stable release
    - 'beta' for the latest beta release
    - 'nightly' for the latest nightly release
    - 'v4.24.0' for a specific version
    - 'https://...' for a direct download URL";

pub static TOOLCHAIN_LINK_HELP: &str = r"DISCUSSION:
    'name' is the custom name to be assigned to the new toolchain.

    'path' specifies the directory where the binaries and libraries for
    the custom toolchain can be found. For example, when used for
    development of Lean itself, toolchains can be linked directly out of
    the Lean build directory. After building, you can test out different
    compiler versions as follows:

        $ lemma toolchain link my-lean <path/to/lean/build>
        $ lemma override set my-lean

    If you now compile a project in the current directory, the custom
    toolchain 'my-lean' will be used.";

pub static TOOLCHAIN_LIST_HELP: &str = r"DISCUSSION:
    Lists all installed toolchains. With the -v/--verbose flag, shows
    additional information including toolchain paths and Lean versions.";

pub static TOOLCHAIN_UNINSTALL_HELP: &str = r"DISCUSSION:
    Uninstalls the specified toolchain. The toolchain must not be the
    active or default toolchain.";

pub static OVERRIDE_HELP: &str = r"DISCUSSION:
    Overrides configure lemma to use a specific toolchain when
    running in a specific directory.

    lemma will automatically select the Lean toolchain specified in
    the `lean-toolchain` file when inside a Lean package, but
    directories can also be assigned their own Lean toolchain manually
    with `lemma override`. When a directory has an override then any
    time `lean` or `lake` is run inside that directory, or one of
    its child directories, the override toolchain will be invoked.

    To pin to a specific version:

        $ lemma override set v4.24.0

    Or to use the stable channel:

        $ lemma override set stable

    To see the active toolchain use `lemma show`. To remove the
    override and use the default toolchain again, `lemma override
    unset`.";

pub static OVERRIDE_SET_HELP: &str = r"DISCUSSION:
    Sets the override toolchain for the current directory (or the
    directory specified with --path). This override will apply to
    all subdirectories as well.";

pub static OVERRIDE_UNSET_HELP: &str = r"DISCUSSION:
    If `--path` argument is present, removes the override toolchain
    for the specified directory. Otherwise, removes the override
    toolchain for the current directory.";

pub static OVERRIDE_LIST_HELP: &str = r"DISCUSSION:
    Lists all directory overrides that have been configured.";

pub static WHICH_HELP: &str = r"DISCUSSION:
    Shows the path to the binary that will be executed when you run
    the specified command. This is useful for debugging which toolchain
    is being used.

    For example:

        $ lemma which lean
        ~/.lemma/toolchains/v4.24.0/bin/lean";

pub static UPDATE_HELP: &str = r"DISCUSSION:
    Updates installed toolchains to their latest versions. If a specific
    toolchain is specified, only that toolchain is updated. Otherwise,
    all updateable toolchains are updated.

    Toolchains that are pinned to specific versions (like 'v4.24.0') are
    skipped during update. Only channel toolchains (like 'stable') are
    automatically updated.";

pub static COMPLETIONS_HELP: &str = r"DISCUSSION:
    One can generate a completion script for `lemma` that is
    compatible with a given shell. The script is output on `stdout`
    allowing one to re-direct the output to the file of their
    choosing. Where you place the file will depend on which shell, and
    which operating system you are using. Your particular
    configuration may also determine where these scripts need to be
    placed.

    Here are some common set ups for the supported shells under
    Unix and similar operating systems (such as GNU/Linux).

    BASH:

    Completion files are commonly stored in `/etc/bash_completion.d/`.
    Run the command:

        $ lemma completions bash > /etc/bash_completion.d/lemma.bash-completion

    This installs the completion script. You may have to log out and
    log back in to your shell session for the changes to take effect.

    BASH (macOS/Homebrew):

    Homebrew stores bash completion files within the Homebrew directory.
    With the `bash-completion` brew formula installed, run the command:

        $ lemma completions bash > $(brew --prefix)/etc/bash_completion.d/lemma.bash-completion

    FISH:

    Fish completion files are commonly stored in
    `$HOME/.config/fish/completions`. Run the command:

        $ lemma completions fish > ~/.config/fish/completions/lemma.fish

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

        $ lemma completions zsh > ~/.zfunc/_lemma

    You must then either log out and log back in, or simply run

        $ exec zsh

    for the new completions to take effect.

    POWERSHELL:

    The powershell completion scripts require PowerShell v5.0+ (which
    comes with Windows 10, but can be downloaded separately for Windows 7
    or 8.1).

    First, check if a profile has already been set

        PS C:\> Test-Path $profile

    If the above command returns `False` run the following

        PS C:\> New-Item -path $profile -type file -force

    Now open the file provided by `$profile` (if you used the
    `New-Item` command it will be
    `%USERPROFILE%\Documents\WindowsPowerShell\Microsoft.PowerShell_profile.ps1`

    Next, we either save the completions file into our profile, or
    into a separate file and source it inside our profile. To save the
    completions into our profile simply use

        PS C:\> lemma completions powershell >> %USERPROFILE%\Documents\WindowsPowerShell\Microsoft.PowerShell_profile.ps1";

pub static INFO_HELP: &str = r"DISCUSSION:
    Shows information about the lemma installation, including version
    and installation paths.";

pub static SELF_HELP: &str = r"DISCUSSION:
    The `self` command is used to manipulate the lemma installation.
    It can update lemma to newer versions, or uninstall lemma entirely.";

pub static SELF_UPDATE_HELP: &str = r"DISCUSSION:
    Updates lemma itself to the latest version. This command will
    download and install the latest release of lemma from the
    configured distribution server.

    The update downloads the appropriate binary for your platform,
    extracts it, and replaces the current lemma installation.

    Supported platforms:
    - x86_64-unknown-linux-gnu (Linux x64 GNU)
    - x86_64-unknown-linux-musl (Linux x64 musl)
    - x86_64-apple-darwin (macOS Intel)
    - aarch64-apple-darwin (macOS Apple Silicon)
    - x86_64-pc-windows-gnu (Windows x64)

    Example:

        $ lemma self update";

pub static SELF_UNINSTALL_HELP: &str = r"DISCUSSION:
    Uninstalls lemma and all installed toolchains. This will remove:
    - All installed Lean toolchains
    - All lemma proxy binaries
    - The entire ~/.lemma directory

    This operation is irreversible. Use with caution.

    Example:

        $ lemma self uninstall
        $ lemma self uninstall -y  # Skip confirmation";
