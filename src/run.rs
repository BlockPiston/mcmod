use std::io::{self, Write};

use clap::{Parser, ValueEnum};
use tokio::fs::{self, File};
use tokio::io::AsyncWriteExt;

use crate::sync::SyncCommand;
use crate::util::{IoResult, Project};

#[derive(Debug, Parser)]
pub struct RunCommand {
    /// The side to run
    #[arg(default_value = "client")]
    pub side: Side,

    /// Whether to fully sync before running
    #[arg(short, long)]
    pub sync: bool,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum Side {
    /// Run client
    Client,
    /// Run server
    Server,
}

impl RunCommand {
    pub async fn run(self, dir: &str) -> IoResult<()> {
        let sync = SyncCommand {
            incremental: !self.sync,
        };
        sync.run(dir).await?;
        let project = Project::new_in(dir)?;
        match self.side {
            Side::Client => {
                project.run_gradlew(&["runClient"]).await?;
            }
            Side::Server => {
                agree_to_eula(&project).await?;
                project.run_gradlew(&["runServer"]).await?;
            }
        }

        Ok(())
    }
}

async fn agree_to_eula(project: &Project) -> IoResult<()> {
    let mut eula_path = project.forge_root();
    eula_path.push("run");
    eula_path.push("eula.txt");
    if eula_path.exists() {
        let content = fs::read_to_string(&eula_path).await?;
        for line in content.lines() {
            if line.trim() == "eula=true" {
                return Ok(());
            }
        }
    }

    let env = std::env::var("MCMOD_EULA_AUTO_AGREE").unwrap_or_default();
    if env == "true" || env == "1" {
        println!("Automatically agreeing to EULA to run the server (because MCMOD_EULA_AUTO_AGREE is set)");
        println!("Please read the EULA at https://account.mojang.com/documents/minecraft_eula");
    } else {
        println!("Agreeing to the EULA is required to launch the server");
        println!("Please read the EULA at https://account.mojang.com/documents/minecraft_eula");
        println!("You can set MCMOD_EULA_AUTO_AGREE=true to automatically agree to the EULA");
        print!("Do you want to agree to the EULA? (y/N) ");
        io::stdout().flush()?;
        let mut buffer = String::new();
        let stdin = io::stdin();
        stdin.read_line(&mut buffer)?;
        if buffer.trim().to_lowercase() != "y" {
            Err(io::Error::new(io::ErrorKind::Other, "EULA not agreed"))?;
        }
    }

    File::create(&eula_path)
        .await?
        .write_all(b"eula=true")
        .await?;

    Ok(())
}
