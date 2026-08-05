#![windows_subsystem = "windows"]

use std::{
    collections::{BTreeMap, HashMap}, process::{self, Stdio}, sync::{
        Arc, Mutex,
        mpsc::{Receiver, Sender},
    }, task, time::Duration,
};

use discord_rich_presence::DiscordIpcClient;
use eframe::{egui, wgpu::rwh::RawDisplayHandle::UiKit};

use egui::{ComboBox, TextBuffer};

use egui_extras::{Column, TableBuilder};

use rfd::FileDialog;
use tokio::runtime::Runtime;
use anyhow::Result;

use ani_search::{
    Config, Edge, Translation, VideoPlayer, WatchStatus, anidb::*, anilist, anilist_search_shows, anilist_user_shows::{self, List}, animeschedule, aria2_download, capitalize_word, check_credentials, create_rpc_client, episodes, get_config, search_anilist, update_discord_status, update_settings, write_to_log,
};

fn main() -> Result<()> {
    let rt = Runtime::new().expect("Unable to create Runtime");
    let _enter = rt.enter();

    std::thread::spawn(move || {
        rt.block_on(async {
            loop {
                tokio::time::sleep(Duration::from_secs(3600)).await;
            }
        })
    });

    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "AniSearch",
        native_options,
        Box::new(|cc| Ok(Box::new(Main::new(cc)))),
    )?;
    Ok(())
}

#[derive(Debug, PartialEq)]
enum SearchProvider {
    
    Anilist,
}

impl Default for SearchProvider {
    fn default() -> Self {
        Self::Anilist
    }
}
#[derive(Debug, Copy, Clone, PartialEq)]
enum Pages {
    Init,
    Main,
    Show,
    APIAuth,
    Settings,
    Anilist,
    #[cfg(debug_assertions)]
    Debug,
}

impl Pages {
    fn is_main_page(&self) -> bool {
        matches!(self, Pages::Main)
    }
    fn is_show_page(&self) -> bool {
        matches!(self, Pages::Show)
    }
}

#[derive(Debug, Default)]
struct Player {
    spawned: bool,
    process_id: u32,
}

struct Main {
    config: Config,
    pages: Pages,
    provider: SearchProvider,
    panel_extended: bool,
    previous_page: Vec<Pages>,
    player_locations: Arc<Mutex<HashMap<String, String>>>,
    discord_rpc_client: Option<DiscordIpcClient>,
    color_theme: egui::Theme,

    anime: String,
    translation: Translation,
    shows: Vec<Edge>,
    show_focus: bool,
    episode_list: episodes::AvailableEpisodesDetail,
    show_info: animeschedule::Anime,
    selected_show: Edge,
    selected_episode: String,
    selected_episode_urls: Vec<(String, Vec<(i32, String)>)>,
    player: Player,

    anilist_info: anilist_user_shows::MediaListCollection,
    anilist_search: anilist_search_shows::Page,
    anilist_selected_show: anilist_search_shows::Medum,
    anilist_show_status: WatchStatus,
    anilist_error: String,
    anilist_progress: anilist_user_shows::Entry,

    ani_db_show: AniDBId,
    ani_db_episodes: Vec<AniDbEpisode>,
    ani_db_episode_quality: BTreeMap<String, BTreeMap<String, Vec<String>>>,

    anidb_tx: Sender<AniDBId>,
    anidb_rx: Receiver<AniDBId>,

    anidb_quality_tx: Sender<BTreeMap<String, BTreeMap<String, Vec<String>>>>,
    anidb_quality_rx: Receiver<BTreeMap<String, BTreeMap<String, Vec<String>>>>,
}

impl Default for Main {
    fn default() -> Self {
        let (anidb_tx, anidb_rx) = std::sync::mpsc::channel();
        let (anidb_quality_tx, anidb_quality_rx) = std::sync::mpsc::channel();
        Self {
            config: Config::default(),
            pages: Pages::Init,
            provider: SearchProvider::default(),
            panel_extended: false,
            previous_page: Vec::new(),
            player_locations: Arc::new(Mutex::new(HashMap::new())),
            discord_rpc_client: None,
            color_theme: egui::Theme::Dark,


            anime: Default::default(),
            translation: Translation::Sub,
            shows: Vec::new(),
            show_focus: false,
            show_info: animeschedule::Anime::default(),
            selected_show: Edge::default(),
            selected_episode: Default::default(),
            selected_episode_urls: Vec::new(),
            episode_list: episodes::AvailableEpisodesDetail::default(),
            player: Player::default(),

            anilist_info: anilist_user_shows::MediaListCollection::default(),
            anilist_search: Default::default(),
            anilist_progress: Default::default(),
            anilist_selected_show: Default::default(),
            anilist_show_status: WatchStatus::None,
            anilist_error: Default::default(),

            ani_db_show: AniDBId::default(),
            ani_db_episodes: Vec::new(),
            ani_db_episode_quality: BTreeMap::new(),

            anidb_tx,
            anidb_rx,
            anidb_quality_tx,
            anidb_quality_rx,
        }
    }
}

impl Main {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        egui_extras::install_image_loaders(&cc.egui_ctx);
        
        let mut slf = Self::default();
        if let Some(storage) = cc.storage
        && let Some(theme) = eframe::get_value(storage, eframe::APP_KEY){
            slf.color_theme = theme;
        }
        slf
    }

    fn back(&mut self) -> Pages {
        if let Some(prev_page) = self.previous_page.pop() {
            prev_page
        } else {
            self.pages
        }
    }

    fn anilist_token(&mut self) -> Option<&str> {
        if !self.config.anilist.token.is_empty() {
            Some(self.config.anilist.token.as_str())
        } else {
            None
        }
    }

    fn spawn_player(
        &self,
        media_title: &str,
        link: &str,
    ) -> Result<process::Child, std::io::Error> {
        let spawn = match self.config.app_settings.video_player {
            VideoPlayer::MPV => process::Command::new("mpv")
                .arg("--tls-verify=no")
                .arg("--cache=yes")
                .arg("--save-position-on-quit=yes")
                .arg(&format!("--force-media-title={}", media_title))
                .arg(link)
                .stderr(Stdio::null())
                .spawn(),
            VideoPlayer::VLC => process::Command::new("vlc")
                .arg(&format!("--meta-title={}", media_title))
                .arg(media_title)
                .arg(link)
                .stderr(Stdio::null())
                .spawn(),
        };
        spawn
    }

    fn anilist_show_status(&mut self, mut item: List, ui: &mut egui::Ui) {
        let status = capitalize_word(&item.name);
        item.entries.sort_by(|a, b| {
            a.media
                .title
                .user_preferred
                .cmp(&b.media.title.user_preferred)
        });
        egui::CollapsingHeader::new(&status).id_salt(&status.clone().to_lowercase()).show(ui, |ui| {   
            // ui.horizontal(|ui|{
            //     if ui.button("^").clicked(){
                    
            //     }
            // });                     
            egui::ScrollArea::vertical().id_salt(format!("{status}_scroll")).max_height(250.0).show(ui, |ui|{
                let selected_state = vec![false; item.entries.len()]; 
                for (i,show) in item.entries.into_iter().enumerate(){
                    if let Some(title) = &show.media.title.english{
                        if ui.selectable_label(selected_state[i], title).clicked(){
                            self.anilist_progress = show.clone();
                            
                            if let Ok(searched_title) = search_anilist(title, &self.anilist_token()){
                                if  let Some(current_page) = searched_title.page_info.current_page{
                                    if current_page == 1{
                                    
                                    dbg!(&searched_title.media);
                                    if let Some(medias) = searched_title.media{
                                    if medias.len() > 0 {
                                        for media in medias{
                                            if let Some(title_searched) = &media.title{
                                                if let Some(user_preferred) = &title_searched.user_preferred{
                                                    if user_preferred == title{
                                                        dbg!(&media);
                                                        if let Some(list_entry) = &media.media_list_entry{
                                                            if let Some(status) = &list_entry.status{
                                                                match status.to_uppercase().as_str(){
                                                                    "COMPLETED"=> self.anilist_show_status = WatchStatus::Completed,
                                                                    "CURRENT"=> self.anilist_show_status = WatchStatus::Watching,
                                                                    "DROPPED"=> self.anilist_show_status = WatchStatus::Dropped,
                                                                    "PAUSED"=> self.anilist_show_status = WatchStatus::Paused,
                                                                    "PLANNING"=> self.anilist_show_status = WatchStatus::Planning,
                                                                    "REPEATING"=> self.anilist_show_status = WatchStatus::Repeating,
                                                                    _ => self.anilist_show_status = WatchStatus::None
                                                                }
                                                            }
                                                        }
                                                        self.anilist_selected_show = media.clone();
                                                        self.show_focus = true;
                                                        self.panel_extended = true;
                                                    }
                                                }
                                            }
                                        }
                                        
                                    } else {
                                        self.anilist_error = "Unable to find show".to_string();
                                    }
                                    }
                                    
                                }
                            }
                            }
                        };
                        
                    }
            }
            });
            
        });
    }
}

impl eframe::App for Main {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, &self.color_theme);
        
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        // if self.config.app_settings.initialized == false{
        //     self.config.app_settings.initialized = true;
        //     update_settings(&self.config);

        // }

        egui::MenuBar::new().ui(ui, |ui| {
            ui.separator();
            ui.menu_button("File", |ui| {
                if ui.button("Settings").clicked() {
                    self.previous_page.push(self.pages);
                    self.pages = Pages::Settings;
                }
                #[cfg(debug_assertions)]
                if ui.button("DEBUG").clicked() {
                    self.pages = Pages::Debug;
                }
                if ui.button("Quit").clicked() {
                    ui.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
            ui.separator();
            ui.menu_button("Anilist", |ui| {
                if ui.button("Get Auth token").clicked() {
                    self.previous_page.push(self.pages);
                    self.pages = Pages::APIAuth;
                }

                ui.add_enabled_ui(!&self.config.anilist.token.is_empty(), |ui| {
                    if ui.button("My Anilist").clicked() {
                        self.previous_page.push(self.pages);
                        self.pages = Pages::Anilist;
                        if let Ok(info) = anilist::get_anime_from_list(
                            self.config.anilist.user_id,
                            &self.anilist_token(),
                        ) {
                            self.anilist_info = info;
                        }
                    }
                })
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::RIGHT), |ui| {
                if self.show_focus == true {
                    if self.panel_extended == true {
                        if ui.button("Close Panel").clicked() {
                            self.panel_extended = !self.panel_extended;
                        }
                    } else {
                        if ui.button("Open Panel").clicked() {
                            self.panel_extended = !self.panel_extended;
                        }
                    }
                }
            });
        });

        egui::Panel::right("right").resizable(true).min_size(ui.ctx().viewport_rect().width()/2.0).show_collapsible(ui, &mut self.panel_extended.clone(), |ui| {

            match self.pages{
            Pages::Init => {
                egui_extras::install_image_loaders(ui.ctx());
            },
            Pages::Main | Pages::Anilist if self.show_focus =>{ 
                        
                        egui_extras::install_image_loaders(ui.ctx());
                        
                        egui::ScrollArea::vertical().id_salt("selected_show_frame").show(ui, |ui|{
                            if self.pages == Pages::Anilist{
                                let progress= self.anilist_progress.progress as f32;
                                
                                if let Some(episode_count) = self.anilist_selected_show.episodes{
                                
                                    let episode_progress = (progress/ (episode_count as f32)) as f32;
                                    ui.add(egui::ProgressBar::new(episode_progress as f32).text(format!("{progress}/{episode_count}")));
                                } else {
                                    if let Some(next_episode_main) = &self.anilist_selected_show.next_airing_episode{
                                        if let Some(next_episode) = next_episode_main.episode{

                                            let episode_progress = progress/(next_episode-1) as f32;
                                            
                                            ui.add(egui::ProgressBar::new(episode_progress as f32).text(format!("{}/{}", progress, (next_episode-1))));
                                        }
                                    } else {
                                        ui.label("Unable to Display Progress");
                                    }
                                }

                                egui::ComboBox::new("show_status", "Status").selected_text(format!("{:?}", self.anilist_show_status)).show_ui(ui, |ui|{
                                    ui.selectable_value(&mut self.anilist_show_status, WatchStatus::Completed, "Completed");
                                    ui.selectable_value(&mut self.anilist_show_status, WatchStatus::Repeating, "Repeating");
                                    ui.selectable_value(&mut self.anilist_show_status, WatchStatus::Watching, "Watching");
                                    ui.selectable_value(&mut self.anilist_show_status, WatchStatus::Paused, "Paused");
                                    ui.selectable_value(&mut self.anilist_show_status, WatchStatus::Dropped, "Dropped");
                                    ui.selectable_value(&mut self.anilist_show_status, WatchStatus::Planning, "Planning");
                                    ui.selectable_value(&mut self.anilist_show_status, WatchStatus::None, "None");         
                                });
                            }
                            
                            if let Some(title) = &self.anilist_selected_show.title &&
                                let Some(user_preferred) = &title.user_preferred{
                                    ui.heading(user_preferred);
                                }
                                
                            
                            ui.horizontal(|ui|{
                                ui.label("Genres: ");
                                ui.separator();
                                if let Some(genres) = &self.anilist_selected_show.genres{
                                    for genre in genres{
                                        ui.label(genre);
                                        ui.separator();
                                    
                                    }
                                }
                                
                            });
                            if let Some(cover_image) = &self.anilist_selected_show.cover_image && let Some(extra_large) = &cover_image.extra_large{
                                    ui.add(egui::Image::new(extra_large)
                                    .max_width(ui.ctx().viewport_rect().width()/4.0)
                                    .maintain_aspect_ratio(true)); 
                            }
                            let mut sub_button_clicked = false ;
                            ui.horizontal(|ui|{
                                if let Some(title) = &self.anilist_selected_show.title{
                                    let english = match &title.english{
                                        Some(e) => e.to_string(),
                                        None => "".to_string()
                                    };
                                    let romanji = match &title.romaji{
                                        Some(r) => r.to_string(),
                                        None => "".to_string()
                                    };
                                    if let Some(user_preferred) = &title.user_preferred && ui.button("Watch Sub").clicked(){
                                        sub_button_clicked = true;
                                        if let Ok(client)=AniDbClient::new(){
                                            let tx = self.anidb_tx.clone();    
                                            let ctx = ui.ctx().clone();
                                            let show_title = user_preferred.clone();
                                                tokio::spawn(async move{
                                                let shows = client.search(&show_title).await.expect("Unable to process query");
                                                if let Some(show ) = shows.iter().find(|x| x.title.clone().unwrap_or_default().to_lowercase() == romanji.clone().to_lowercase() || x.title.clone().unwrap_or_default().to_lowercase() == english.clone().to_lowercase()){
                                                            dbg!(&show);
                                                            let _ = tx.send(show.clone());
                                                }
                                                ctx.request_repaint();
                                            });
                                        }

                                    


                                    // if ui.button("Watch Dub").clicked() && let Ok(shows) = get_shows(&user_preferred, &Translation::Dub, &self.config){
                                    //         for show in shows{
                                    //             if show.name.to_lowercase() == user_preferred.to_ascii_lowercase(){
                                    //                 self.selected_show = show.clone();
                                    //                     if let Ok(eps) = get_episode_list(&show.id){
                                    //                     self.episode_list = eps.clone();
                                    //                 }
                                    //                 self.previous_page.push(self.pages);
                                    //                 self.pages= Pages::Show;
                                    //                 self.show_focus = false;
                                                    
                                    //             }
                                    //         }
                                            
                                    //     }
                                    }
                                }                                           
                                
                                
                            });
                                    if let Ok(show) = self.anidb_rx.try_recv(){
                                        self.ani_db_show = show;
                                    }
                                        let title = match &self.ani_db_show.title{
                                            Some(t) => t,
                                            None => &"No Title Available".to_string()
                                        };

                                            if let Some(episodes) = &self.ani_db_show.episodes.clone(){
                                                let selected = vec![false; episodes.len()];
                                                for (i, episode) in episodes.iter().enumerate(){

                                                    ui.horizontal(|ui|{
                                                        if ui.selectable_label(selected[i], format!("Episode: {}", episode.number)).clicked(){

                                                            self.selected_episode = episode.number.to_string();
                                                            if let Ok(client) = AniDbClient::new(){
                                                                let tx = self.anidb_quality_tx.clone();
                                                                let ctx = ui.ctx().clone();
                                                                let episode_id = episode.id;
                                                                tokio::spawn(async move{
                                                                    let episode_links= client.get_episode_m3u8(episode_id).await.expect("unable to get episodes");
                                                                    dbg!(&episode_links);
                                                                    let _ = tx.send(episode_links);
                                                                    ctx.request_repaint();
                                                                });
                                                            } 
                                                        }
                                                        if episode.filler{
                                                            ui.label("(Filler Episode)");
                                                        }
                                                    });
                                                    if self.selected_episode == episode.number.to_string(){


                                                        if let Ok(episode_quality) = self.anidb_quality_rx.try_recv(){
                                                            self.ani_db_episode_quality = episode_quality;
                                                        }
                                                        for language in self.ani_db_episode_quality.clone(){
                                                            match self.translation{
                                                                Translation::Sub if language.0 =="jpn"=> {
                                                                    
                                                                        ui.horizontal(|ui|{
                                                                            let selected = vec![false; language.1.len()];
                                                                            for (i, quality) in language.1.iter().enumerate(){
                                                                                let link = quality.1[0].clone();
                                                                                if ui.selectable_label(selected[i], quality.0).clicked(){
                                                                                    
                                                                                    let show_title = match &self.ani_db_show.title{
                                                                                        Some(title) => title,
                                                                                        None => &"".to_string()
                                                                                    };
                                                                                    let media_title = format!("\"{} Episode {}\"", show_title, &self.selected_episode);    
                                                                                    if self.player.spawned && self.player.process_id > 0{
                                                                                        #[cfg(unix)]
                                                                                        match process::Command::new("kill").args(["-9", &self.player.process_id.to_string()]).output(){
                                                                                            Ok(process) => () ,
                                                                                            Err(e) => ()
                                                                                        };   

                                                                                    }
                                                                                    
                                                                                    match self.spawn_player(&media_title, &link){
                                                                                        Ok(spawned) => {
                                                                                            match &mut self.discord_rpc_client{
                                                                                                Some(client) => {
                                                                                                    update_discord_status(client, show_title);
                                                                                                }
                                                                                                None => ()
                                                                                            }
                                                                                            self.player.spawned= true;
                                                                                            self.player.process_id = spawned.id();
                                                                                            if self.anilist_token().is_some() && let Some(show_id) = self.anilist_selected_show.id{
                                                                                                    let duration = self.anilist_selected_show.duration.unwrap_or_default();
                                                                                                
                                                                                                    if duration.to_string() != self.selected_episode{
                                                                                                        anilist::update_progress(&show_id, Some(&self.selected_episode.clone()), anilist::WatchStatus::Watching, &self.anilist_token().clone());
                                                                                                    }else{
                                                                                                        anilist::update_progress(&show_id, Some(&self.selected_episode.clone()), anilist::WatchStatus::Completed, &self.anilist_token().clone());
                                                                                                    }
                                                                                                }
                                                                        


                                                                                        },
                                                                                        Err(e) => {dbg!(e);}

                                                                                    }

                                                                                }
                                                                                if ui.button("⭳").clicked(){
                                                                                    let download_location = self.config.app_settings.download_dir.clone();
                                                                                    let url = &link.clone();
                                                                                    tokio::spawn(async move{
                                                                                        aria2_download(&download_location, &url).await.expect("Unable to download");
                                                                                    });
                                                                                    
                                                                                }
                                                                            }
                                                                        });

                                                                    
                                                                },
                                                                Translation::Dub => {

                                                                },
                                                                _ => ()
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                            egui::CollapsingHeader::new("Description").id_salt("show_description").show(ui, |ui|{
                            if let Some(description) = &self.anilist_selected_show.description{
                                ui.label(description.replace("<BR>", "").replace("<br>", ""));
                            
                            }
                            if let Some(score) = &self.anilist_selected_show.average_score{
                                ui.label(format!("Average Score: {}", score));
                            } else {
                                ui.label("No Score Available");
                            }
                            if let Some(season) =&self.anilist_selected_show.season && let Some(season_year) = &self.anilist_selected_show.season_year{
                                ui.label(format!("Season: {} {}",season,season_year));
                            
                            }
                            if let Some(next_episode) = &self.anilist_selected_show.next_airing_episode{
                                if let Some(next_airing) = next_episode.time_until_airing{
                                    let airdate = chrono::Utc::now() - chrono::Duration::seconds(next_airing);                                
                                    ui.label(format!("Next Episode Release: {} UTC",airdate.date_naive()));
                                
                                }
                                
                                
                            } else {
                                ui.label("No more additional episodes airing.");
                            }

                        });

                            
                            
                        });
                        
                    },
            _ => ()
        }
    });

        egui::CentralPanel::default().show(ui, |ui| match self.pages{
            
            Pages::Init=>{
                
                dotenvy::dotenv().expect("Unable to load .env file");
                
                if let Ok(discord_client_id) = std::env::var("DISCORD_CLIENT_ID"){
                    if let Ok(discord_client) = create_rpc_client(&discord_client_id){
                        self.discord_rpc_client = Some(discord_client);
                        
                    }
                    
                }

                if !self.config.app_settings.initialized && let Ok(config) = get_config(){
                        self.config = config;
                        self.config.app_settings.initialized = true;
                        update_settings(&self.config);
                
                }
                let player_locations = Arc::new(Mutex::new(HashMap::new()));
                for player in ["mpv", "vlc"]{
                    let map_clone = Arc::clone(&player_locations);
                    tokio::spawn(async move {
                        use ani_search::find_player;
                        let location = find_player(player).await.expect("failed");
                        let mut map = map_clone.lock().expect("Unable to lock Mutex");
                        map.insert(player.to_string(), location);
                        
                    });
                }
                
                
                self.player_locations = player_locations.clone();
                dbg!(&self.player_locations);
                self.pages = Pages::Main;

            }
            Pages::Main => {
                ui.horizontal(|ui|{
                    
                    let search_bar = ui.add(egui::TextEdit::singleline(&mut self.anime).hint_text("Search for show..."));
                    
    
                    if (ui.button("Search").clicked() || 
                    (search_bar.lost_focus() 
                    && ui.input(|i| i.key_pressed(egui::Key::Enter)))) 
                    && let Ok(pages) = search_anilist(&self.anime.clone(), &self.anilist_token().clone())
                    {
                            self.anilist_search = pages;
                            
                        }
                });
                        if let Some(current_page) = self.anilist_search.page_info.current_page {
                            let page_info = &self.anilist_search.page_info;
                            let last_page = page_info.last_page;

                            egui::ScrollArea::vertical().id_salt("anilist_search").show(ui, |ui|{
                                if let Some(medias) = &self.anilist_search.media{
                                    let selected_state = vec![false; medias.len()];
                                    for (i, media) in medias.into_iter().enumerate(){
                                        if let Some(title) = &media.title{
                                            if let Some(user_preferred) = &title.user_preferred{
                                            if ui.selectable_label(selected_state[i], user_preferred).clicked(){
                                                self.anilist_selected_show = media.clone();
                                                if self.ani_db_show.episodes.is_some(){
                                                   self.ani_db_show.episodes = None;
                                                }
                                                self.show_focus = true;
                                                dbg!(media.clone());
                                                self.panel_extended = true;
                                        }
                                            }
                                        }
                                        
                                    }
                                }
                            });
                            
                            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui|{
                                
                                if page_info.has_next_page == Some(true){
                                    ui.horizontal(|ui|{
                                        ui.add_enabled_ui(Some(current_page) != Some(1), |ui|{
                                            if ui.button("<").clicked(){
                                                    
                                            }
                                        });
                                        ui.add_enabled_ui(Some(current_page) != last_page, |ui| {
                                            if ui.button(">").clicked(){
                                                
                                            }
                                        });

                                    });   
                                }
 
                            });
                            
                        
                        }
                
            },
            Pages::Show => {
                if self.panel_extended{
                    self.panel_extended = false;
                }
                ui.horizontal(|ui|{
                    if ui.selectable_label(self.pages.is_main_page(), "<").clicked(){
                        self.pages = self.back();
                        ui.ctx().request_repaint();
                    }
                    ui.heading(&self.selected_show.name);

                    ComboBox::new("translation", "Translation Type").selected_text(format!("{:?}", self.translation)).show_ui(ui, |ui|{
                            ui.selectable_value(&mut self.translation, Translation::Sub, "Sub");
                            ui.selectable_value(&mut self.translation, Translation::Dub, "Dub");
                        }
                    );
                    
                });
                  
                
                if let Some(description) = &self.show_info.description{
                    ui.label(description);
                } else {
                    ui.label("No available description for this show.");
                }
                
                ui.horizontal(|ui|{
                    ui.label("Genre(s): ");
                    for genre in &self.show_info.genres{
                        ui.label(genre.name.clone());
                        ui.separator();
                    }
                });
                
                    
                //self.play_translation(self.translation, ui);
                
            },
            Pages::APIAuth =>{
                ui.horizontal(|ui|{
                if ui.button("<").clicked(){
                        self.pages = self.back();
                    }
                ui.heading("API Authorization");
                });
                ui.label("Connect your accounts to ani-search for syncronized viewing.");
                ui.label("Currently supported: Anilist");

                ui.separator();

                ui.horizontal(|ui|{
                    ui.label("Please");
                    ui.hyperlink_to("request and AniList Auth Token", "https://anilist.co/api/v2/oauth/authorize?client_id=9857&response_type=token");
                    ui.label("and paste the token in the below section: ");
                });
                

                egui::ScrollArea::both().id_salt("auth_token").max_height(300.0).show(ui, |ui|{
                    ui.add(egui::TextEdit::multiline(&mut self.config.anilist.token).desired_width(300.0)
                    .hint_text("Access Token..."));
                });
                //dbg!(&self.config.anilist.token);
                
                if ui.button("Submit").clicked(){
                    
                    match check_credentials(&self.anilist_token()){
                        Ok(id) => self.config.anilist.user_id = id,
                        Err(_) => {write_to_log("Unable to get User ID",ani_search::MessageType::Error); }
                    }
                    update_settings(&self.config);
                    self.previous_page.push(self.pages);
                    self.pages = Pages::Main;
                }

            },
            Pages::Settings => {
                ui.horizontal(|ui|{
                if ui.button("<").clicked(){
                      self.pages = self.back();  
                    }
                    ui.heading("AniSearch Settings");
                ui.separator();

                });
                //dbg!(&self.config.app_settings);
                
                egui::Grid::new("settings")
                .num_columns(2)
                .spacing([40.0, 10.0])
                .show(ui, |ui| {
                    ui.label("Theme");
                    let prev_theme= ui.theme();
                    egui::widgets::global_theme_preference_buttons(ui);
                    if prev_theme != ui.theme(){
                        self.color_theme = ui.theme();
                    }
                    ui.end_row();

                    ui.label("Allow Adult");
                    ui.add(egui::Checkbox::without_text(&mut self.config.app_settings.allow_adult));
                    ui.end_row();
                    
                    
                    ui.label("Selected Media Player");
                    egui::ComboBox::new("media_player", "").selected_text(format!("{}",&self.config.app_settings.video_player)).show_ui(ui, |ui|{
                            if let Ok(player_locations) = self.player_locations.lock(){
                                if player_locations.get("mpv").is_some(){
                                    ui.selectable_value(&mut self.config.app_settings.video_player, VideoPlayer::MPV, "MPV");
                                }
                                if player_locations.get("vlc").is_some(){
                                    ui.selectable_value(&mut self.config.app_settings.video_player, VideoPlayer::VLC, "VLC");
                        
                                }
                                
                            }        
                        }) ;
                    ui.end_row();

                    ui.label("Download Directory");
                    ui.horizontal(|ui|{
                        ui.add(egui::TextEdit::singleline(&mut self.config.app_settings.download_dir));
                        if ui.button("📁").clicked(){
                                if let Some(folder) = FileDialog::new().set_directory("/").pick_folder(){ 
                                    self.config.app_settings.download_dir = match folder.to_str(){
                                        Some(f) => f.to_string(),
                                        None => "Unable to get Folder Path".to_string()
                                    }
                                }                                

                            }
                        });
                            
                    

            });     


                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui|{
                    ui.horizontal(|ui|{
                        if ui.button("Save").clicked(){
                            update_settings(&self.config);
                            self.previous_page.push(self.pages);
                            self.pages = Pages::Main;
                        }
                        if ui.button("Apply").clicked(){
                            update_settings(&self.config);
                        }
                        
                    });
                    
                });
            },
            Pages::Anilist => {
                let user = &self.anilist_info.user.clone();
                ui.horizontal(|ui|{
                    if ui.button("<").clicked(){
                        self.pages = self.back();
                    }
                    ui.heading(format!("{}'s Anilist Information", user.name));
                    ui.image(&user.avatar.large);
                });

                ui.separator();

                let media_list = &self.anilist_info.lists.clone();
                
                
                for item in media_list{
                    match item.name.to_lowercase().as_str() {
                        "watching" => {
                            self.anilist_show_status(item.clone(), ui);
                        },
                        "planning" => {
                            self.anilist_show_status(item.clone(), ui);
                        },
                        "completed" => {
                            self.anilist_show_status(item.clone(), ui);
                        },
                         _=> ()
                    }    
                }
                
            },
            #[cfg(debug_assertions)]
            Pages::Debug=>{
                if ui.button("test theme").clicked(){
                    dbg!(&self.color_theme);         
                }
            }           
        });
    }
}
