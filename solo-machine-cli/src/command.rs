mod bank;
mod chain;
mod ibc;

use std::{
    fmt::Display,
    io::{stdout, Write},
};

use anyhow::{anyhow, ensure, Context, Result};
use bip39::{Language, Mnemonic};
use cli_table::{Cell, Row, RowStruct, Style};
use solo_machine::{signer::MnemonicSigner, DbPool, Signer, MIGRATOR};
use structopt::{clap::Shell, StructOpt};
use termcolor::{ColorChoice, ColorSpec, StandardStream, WriteColor};

use self::{bank::BankCommand, chain::ChainCommand, ibc::IbcCommand};

#[derive(Debug, StructOpt)]
#[structopt(
    name = "solo-machine-cli",
    about = "A command line interface for IBC solo machine"
)]
pub struct Command {
    /// Does not print styled/colored statements
    #[structopt(long)]
    no_style: bool,
    /// Database connection string
    #[structopt(long, env = "SOLO_DB_PATH", hide_env_values = true)]
    db_path: Option<String>,
    /// Prefix for bech32 addresses used on solo machine
    #[structopt(long, env = "SOLO_ACCOUNT_PREFIX", hide_env_values = true)]
    account_prefix: Option<String>,
    /// Mnemonic phrase for account on IBC enabled chain
    #[structopt(long, env = "SOLO_MNEMONIC", hide_env_values = true)]
    mnemonic: Option<String>,
    /// HD wallet path to be used when deriving public key from mnemonic
    #[structopt(
        long,
        default_value = "m/44'/118'/0'/0/0",
        env = "SOLO_HD_PATH",
        hide_env_values = true
    )]
    hd_path: String,
    #[structopt(subcommand)]
    subcommand: SubCommand,
}

#[derive(Debug, StructOpt)]
#[allow(clippy::large_enum_variant)]
pub enum SubCommand {
    /// Bank operations (e.g. mint, burn, check balance, etc.)
    Bank(BankSubCommand),
    /// Chain operations (managing chain state and metadata)
    Chain(ChainSubCommand),
    /// Generate completion scripts for solo-machine-cli
    GenCompletion {
        #[structopt(long, default_value = "bash")]
        shell: Shell,
    },
    /// Used to connect, send tokens and receive tokens over IBC
    Ibc(IbcSubCommand),
    /// Initializes database for solo machine
    Init,
}

#[derive(Debug, StructOpt)]
pub struct BankSubCommand {
    #[structopt(subcommand)]
    subcommand: BankCommand,
}

#[derive(Debug, StructOpt)]
pub struct ChainSubCommand {
    #[structopt(subcommand)]
    subcommand: ChainCommand,
}

#[derive(Debug, StructOpt)]
pub struct IbcSubCommand {
    #[structopt(subcommand)]
    subcommand: IbcCommand,
}

impl Command {
    pub async fn execute(self) -> Result<()> {
        let color_choice = if self.no_style {
            ColorChoice::Never
        } else {
            ColorChoice::Auto
        };

        match self.subcommand {
            SubCommand::Bank(bank) => {
                ensure!(
                    self.account_prefix.is_some(),
                    "`account-prefix` is required for bank commands"
                );
                ensure!(self.db_path.is_some(), "`db-path` is required");

                let db_pool = get_db_pool(&self.db_path.unwrap()).await?;
                let signer = get_signer(self.mnemonic, self.hd_path)?;

                bank.subcommand
                    .execute(db_pool, signer, &self.account_prefix.unwrap(), color_choice)
                    .await
            }
            SubCommand::Chain(chain) => {
                ensure!(self.db_path.is_some(), "`db-path` is required");

                let db_pool = get_db_pool(&self.db_path.unwrap()).await?;
                let signer = get_signer(self.mnemonic, self.hd_path)?;

                chain
                    .subcommand
                    .execute(db_pool, signer, color_choice)
                    .await
            }
            SubCommand::GenCompletion { shell } => {
                Self::clap().gen_completions_to("solo-machine-cli", shell, &mut stdout());
                Ok(())
            }
            SubCommand::Ibc(ibc) => {
                ensure!(self.db_path.is_some(), "`db-path` is required");

                let db_pool = get_db_pool(&self.db_path.unwrap()).await?;
                let signer = get_signer(self.mnemonic, self.hd_path)?;

                ibc.subcommand.execute(db_pool, signer, color_choice).await
            }
            SubCommand::Init => {
                ensure!(self.db_path.is_some(), "`db-path` is required");

                let db_path = self.db_path.unwrap();
                create_db_file(&db_path).await?;

                let db_pool = get_db_pool(&db_path).await?;

                MIGRATOR
                    .run(&db_pool)
                    .await
                    .context("unable to run migrations")?;

                let mut stdout = StandardStream::stdout(color_choice);
                print_stream(
                    &mut stdout,
                    ColorSpec::new().set_bold(true),
                    "Initialized solo machine!",
                )
            }
        }
    }
}

async fn get_db_pool(db_path: &str) -> Result<DbPool> {
    DbPool::connect(&format!("sqlite:{}", db_path))
        .await
        .context("unable to connect to database")
}

fn get_signer(mnemonic: Option<String>, hd_path: String) -> Result<impl Signer> {
    match mnemonic {
        None => Err(anyhow!("currently, only mnemonic signer is supported")),
        Some(ref mnemonic) => Ok(MnemonicSigner {
            mnemonic: Mnemonic::from_phrase(mnemonic, Language::English)
                .context("invalid mnemonic")?,
            hd_path,
        }),
    }
}

async fn create_db_file(db_path: &str) -> Result<()> {
    tokio::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .open(db_path)
        .await
        .map(|_| ())
        .context("unable to create db file")
}

fn add_row(table: &mut Vec<RowStruct>, title: &str, value: impl Display) {
    table.push(vec![title.cell().bold(true), value.cell()].row());
}

fn print_stream(
    stdout: &mut StandardStream,
    color_spec: &ColorSpec,
    s: impl Display,
) -> Result<()> {
    stdout.set_color(color_spec)?;
    writeln!(stdout, "{}", s).context("unable to write to stdout")?;
    stdout.reset().context("unable to reset stdout")?;

    Ok(())
}