use aria2_core::config::{OptionValue,ConfigManager};
use aria2_core::request::{request_group::DownloadOptions,request_group_man::RequestGroupMan};
use anyhow::Result;
use crate::Config;

pub async fn aria2_download(download_dir: &str, url: &str) -> Result<()>{
    let mut aria_config = ConfigManager::new();

    aria_config.set_global_option("dir", 
    OptionValue::Str(download_dir.into())).await?;
    aria_config.set_global_option("split", OptionValue::Int(4)).await?;

    let man =RequestGroupMan::new();
    let opts = DownloadOptions{
        split: Some(4),
        ..Default::default()
    };

    match man.add_group(vec![url.into()], opts).await{
        Ok(gid) => {println!("Download started: #{}", gid.value())},
        Err(e) => eprintln!("Error: {e}")
    }


    Ok(())
}