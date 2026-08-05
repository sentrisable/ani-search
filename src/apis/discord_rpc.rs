use discord_rich_presence::{activity, DiscordIpc, DiscordIpcClient};
use anyhow::Result;

pub fn create_rpc_client(client_id: &str) -> Result<DiscordIpcClient>{
    let client = DiscordIpcClient::new(client_id);
    dbg!(&client);
    Ok(client)
}

pub fn update_discord_status(client: &mut DiscordIpcClient, show: &str) -> Result<()>{
    match client.connect(){
        Ok(_) => {
            let payload = activity::Activity::new()
            .details(format!("Watching {show}"))
            .state("with AniSearch");
            match client.set_activity(payload){
                Ok(_) => {dbg!("Activity Set");},
                Err(_) => {dbg!("Failed to Set Activity");}
            }
            

        },
        Err(_)=> ()
    }
    
    Ok(())
}