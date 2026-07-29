#![windows_subsystem = "windows"]

use std::{time::Duration, sync::{mpsc::{Receiver, Sender}, Arc, Mutex}, task, process};

#[cfg(debug_assertions)]
use ani_search::get_allanime_key;
use eframe::{egui, wgpu::rwh::RawDisplayHandle::UiKit};

use egui::{ComboBox, TextBuffer};

use egui_extras::{Column, TableBuilder};

use tokio::runtime::Runtime;


use ani_search::{
    anidb::*,
    AnilistVariables, AppSettings, AvailableEpisodes, Config, Edge, Translation, WatchStatus, anilist, anilist_search_shows, anilist_user_shows::{self, List}, animeschedule, capitalize_word, check_credentials, compare_names, episode_source, episodes, generate_link, get_config, get_episode_list, get_episode_url, get_next_episode_release, get_shows, search_anilist, update_settings, write_to_log,

};

fn main() {
    let rt = Runtime::new().expect("Unable to create Runtime");
    let _enter = rt.enter();

    
    
    std::thread::spawn(move || {
        rt.block_on(async {
            loop{
                tokio::time::sleep(Duration::from_secs(3600)).await;
            }
        })
    });

    let native_options = eframe::NativeOptions::default();
    eframe::run_native("AniSearch", native_options, Box::new(|cc| Ok(Box::new(Main::new(cc)))));
}

#[derive(Debug, PartialEq)]
enum SearchProvider{
    AllAnime,
    Anilist
}

impl Default for SearchProvider{
    fn default() -> Self {
        Self::Anilist
    }
}
#[derive(Debug, Copy, Clone, PartialEq)]
enum Pages{
    Init,
    Main,
    Show,
    APIAuth,
    Settings,
    Anilist,
    #[cfg(debug_assertions)]
    Debug
}

impl Pages{
    fn is_main_page(&self) -> bool {
        matches!(self, Pages::Main)
    }
    fn is_show_page(&self) -> bool {
        matches!(self, Pages::Show)
    }

}

#[derive(Debug, Default)]
struct Player{
    spawned: bool,
    process_id: u32
}

struct Main{
    config: Config,
    pages: Pages,
    provider: SearchProvider,
    panel_extended: bool,
    previous_page : Pages,


    anime: String,
    translation: Translation,
    shows: Vec<Edge>,
    show_focus: bool,
    episode_list: episodes::AvailableEpisodesDetail,
    show_info: animeschedule::Anime,
    selected_show: Edge,
    selected_episode: String,
    selected_episode_urls: Vec<(String,Vec<(i32,String)>)>,
    player: Player,

    anilist_info: anilist_user_shows::MediaListCollection,
    anilist_search: anilist_search_shows::Page,
    anilist_selected_show : anilist_search_shows::Medum,
    anilist_show_status: WatchStatus,
    anilist_error : String,
    anilist_progress: anilist_user_shows::Entry,
    tx: Sender<String>,
    rx: Receiver<String>,
}

impl Default for Main{
    fn default() -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        Self {
            config: Config::default(),
            pages: Pages::Init,
            provider: SearchProvider::default(),
            panel_extended: false,
            previous_page: Pages::Main,

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


            tx, 
            rx 
        }
    }
}

impl Main{
    fn new(cc: &eframe::CreationContext<'_>)->Self{
        Self::default()
    }

    fn play_translation(&mut self, translation: Translation, ui: &mut egui::Ui){
        let heading = match translation{
            Translation::Sub => "Subbed",
            Translation::Dub => "Dubbed",
            Translation::Raw => "Raw"
        };

        let episodes = match translation{
            Translation::Sub => self.episode_list.sub.clone(),
            Translation::Dub => self.episode_list.dub.clone(),
            Translation::Raw => self.episode_list.raw.clone()
        };
        if !episodes.is_empty(){
            ui.heading(format!("{heading} Episodes"));
            let selected_states = vec![false;episodes.clone().len()];
            egui::ScrollArea::vertical().id_salt(heading).show(ui, |ui|{

            
            for (i,episodes) in episodes.iter().rev().enumerate(){
                let is_selected = selected_states[i];
                if let Some(ep) = episodes{
                    if ui.selectable_label(is_selected, format!("Episode: {ep}")).clicked(){
                        self.selected_episode = ep.clone();
                        match get_episode_url(&self.selected_show.id, &"sub".to_string(), ep){
                        Ok(urls) =>{
                            self.selected_episode_urls = urls.clone();
                            //let child =process::Command::new("mpv").arg(&urls[2].source_url.clone()).spawn().expect("Failed to start mpv");

                        },
                        Err(e) => {
                            println!("{e}");
                        } 
                        
                    }
                    }
                    // dbg!(&self.selected_episode);
                    // dbg!(&ep);
                    if self.selected_episode == ep.clone(){
                        if self.selected_episode_urls.is_empty(){
                                ui.label("Collecting Episode Links...");
                            }else {

                            
                        ui.horizontal(|ui|{
                            ui.label("Sources: ");
                            let selected_url = vec![false; self.selected_episode_urls.clone().len()];
                            for (i,url) in self.selected_episode_urls.clone().iter().enumerate(){
                                let source_name = &url.0;
                                    match source_name.as_str(){
                                        // "Fm-Hls" will add later once set up
                                        "Mp4"|  "S-mp4" | "Yt-mp4" => {
                                            
                                            if ui.selectable_label(selected_url[i], match source_name.as_str(){
                                                "Mp4" =>  "Mp4Upload",
                                                "Fm-Hls" => "Filemoon",
                                                "S-mp4" => "Sharepoint", 
                                                "Yt-mp4" => "Yt",
                                                _=> "No Source available"
                                            }).clicked(){
                                                let episode_link = &url.1[0].1;
                                                    let media_title_arg = format!("--force-media-title=\"{}\": Episode {}", self.selected_show.name, self.selected_episode);
                                                    let mut refer_flag_arg = String::new();
                                                    match source_name.as_str() {
                                                        "Mp4" => refer_flag_arg = "--referrer=https://www.mp4upload.com".to_string(),
                                                        _ => ()

                                                    }
                                                    if self.player.spawned == true && self.player.process_id > 0{
                                                        #[cfg(unix)]
                                                        match process::Command::new("kill").args(&["-9", &self.player.process_id.to_string()]).output(){
                                                            Ok(process) => () ,
                                                            Err(e) => ()
                                                        };   
                                                        
                                                    }
                                                    match process::Command::new("mpv").arg("--tls-verify=no").arg(media_title_arg).arg(refer_flag_arg).arg(episode_link).spawn(){
                                                        Ok(spawned) => {
                                                            self.player.spawned= true;
                                                            self.player.process_id = spawned.id();
                                                            if self.previous_page == Pages::Anilist && !self.config.anilist.token.is_empty(){
                                                            dbg!(&self.anilist_selected_show);
                                                            if let Some(show_id) = &self.anilist_selected_show.id{
                                                                if self.anilist_selected_show.duration.unwrap().to_string() != self.selected_episode{
                                                                    anilist::update_progress(&show_id, Some(&self.selected_episode), anilist::WatchStatus::Watching, &self.config);
                                                                }else{
                                                                    anilist::update_progress(&show_id, Some(&self.selected_episode), anilist::WatchStatus::Completed, &self.config);
                                                                }

                                                                    
                                                                
                                                            }
                                                            
                                                        }
                                                            
                                                        },
                                                        Err(e) => {dbg!(e); ()}
                                                    }
                                                
                                            }
                                            ui.separator();
                                        },
                                        _=>{}
                                    }
                                
                                
                                    
                                    //
                                    //println!("{}",url.source_url);
                                    //let refer_flag_arg = "--referrer=https://www.mp4upload.com";

                                    //let spawn_player = process::Command::new("mpv").arg("--tls-verify=no").arg(media_title_arg).arg(&url.source_url.clone()).spawn();
                                    //match spawn_player{
                                    //    Ok(child) => {},
                                    //    Err(e)=> {println!("{e}");}
                                    //}
                                };
                                
        
                        });
                        
                    }
                }
                    }
            }

            if self.player.spawned == true{
                ui.label(format!("Now Playing Episode {}.", self.selected_episode));
            }
        });
        } else {
            ui.label("No available episodes for selected translation.");
        }
                        
    }

    fn anilist_show_status(&mut self, mut item: List, ui: &mut egui::Ui){
        let status = capitalize_word(&item.name);
        item.entries.sort_by(|a,b| a.media.title.user_preferred.cmp(&b.media.title.user_preferred));
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
                            if let Ok(searched_title) = search_anilist(title, &self.config){
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

impl eframe::App for Main{
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame){
        
        
        // if self.config.app_settings.initialized == false{    
        //     self.config.app_settings.initialized = true;
        //     update_settings(&self.config);
            
        // }
        
        
        egui::MenuBar::new().ui(ui, |ui|{
            ui.separator();
            ui.menu_button("File", |ui|{
                if ui.button("Settings").clicked(){
                    self.previous_page = self.pages;
                    self.pages = Pages::Settings;
                }
                #[cfg(debug_assertions)]
                if ui.button("DEBUG").clicked(){
                    self.pages = Pages::Debug;
                }
                if ui.button("Quit").clicked(){
                    ui.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
            ui.separator();
            ui.menu_button("Anilist", |ui|{
                if ui.button("Get Auth token").clicked(){
                    
                    self.previous_page = self.pages;
                    self.pages = Pages::APIAuth;
                }

                ui.add_enabled_ui(!&self.config.anilist.token.is_empty(), |ui|{
                    if ui.button("My Anilist").clicked(){
                        self.previous_page = self.pages;
                        self.pages = Pages::Anilist;
                        if let Ok(info) = anilist::get_anime_from_list(self.config.anilist.user_id, &self.config.anilist.token){
                            self.anilist_info = info;
                        }
                    }
                })
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::RIGHT), |ui|{
                if self.show_focus == true{
                    if self.panel_extended == true{
                        if ui.button("Close Panel").clicked(){
                            self.panel_extended = !self.panel_extended;
                        }
                    } else {
                        if ui.button("Open Panel").clicked(){
                            self.panel_extended = !self.panel_extended;
                        }
                    }
                }
                
            });
        });

        egui::Panel::right("right").resizable(true).min_size(ui.ctx().viewport_rect().width()/2.0).show_collapsible(ui, &mut self.panel_extended, |ui| {

            match self.pages{
            Pages::Init => {
                egui_extras::install_image_loaders(ui.ctx());
            },
            Pages::Main | Pages::Anilist=> {
                if self.provider == SearchProvider::Anilist{
                    if self.show_focus == true {
                        
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
                            
                            if let Some(title) = &self.anilist_selected_show.title{
                                if let Some(user_preferred) = &title.user_preferred{
                                    ui.heading(user_preferred);
                                }
                                
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
                            if let Some(cover_image) = &self.anilist_selected_show.cover_image{
                                if let Some(extra_large) = &cover_image.extra_large{
                                    ui.add(egui::Image::new(extra_large)
                                    .max_width(ui.ctx().viewport_rect().width()/4.0)
                                    .maintain_aspect_ratio(true));

                                }
                            }
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

                            ui.horizontal(|ui|{
                                if let Some(title) = &self.anilist_selected_show.title{
                                    if let Some(romaji) = &title.romaji{
                                    if ui.button("Watch Sub").clicked(){
                                        
                                        if let Ok(shows) = get_shows(&romaji, &Translation::Sub, &self.config){
                                            for show in shows{
                                                dbg!(&show);
                                                dbg!(&romaji);
                                                
                                                if compare_names(&show, &romaji) == true{
                                                    self.selected_show = show.clone();
                                                        if let Ok(eps) = get_episode_list(&show.id){
                                                            dbg!(&eps);
                                                            self.episode_list = eps.clone();
                                                        }
                                                        self.previous_page = self.pages;
                                                        self.pages= Pages::Show;
                                                        self.show_focus = false;
                                                
                                                }
                                                
                                            }
                                        }
                                    }
                                    if ui.button("Watch Dub").clicked(){
                                        if let Ok(shows) = get_shows(&romaji, &Translation::Dub, &self.config){
                                            for show in shows{
                                                if &show.name.to_lowercase() == &romaji.to_ascii_lowercase(){
                                                    self.selected_show = show.clone();
                                                        if let Ok(eps) = get_episode_list(&show.id){
                                                        self.episode_list = eps.clone();
                                                    }
                                                    self.previous_page = self.pages;
                                                    self.pages= Pages::Show;
                                                    self.show_focus = false;
                                                    
                                                }
                                            }
                                            
                                        }
                                    } 
                                        }
                                        
                                }
                                
                            });
                            
                            
                        });
                        
                    }
                }
            },
            Pages::APIAuth => (),
            Pages::Settings => (),
            Pages::Show => (),
                #[cfg(debug_assertions)]
            Pages::Debug => (),
        }
    });


        egui::CentralPanel::default().show(ui, |ui| match self.pages{
            
            Pages::Init=>{
                egui_extras::install_image_loaders(ui.ctx());
                if self.config.app_settings.initialized == false{
                    if let Ok(config) = get_config(){
                        self.config = config;
                        self.config.app_settings.initialized = true;
                        update_settings(&self.config);
                    }   
                }
                self.pages = Pages::Main;

            }
            Pages::Main => {
                ComboBox::new("provider", "Search Provider").selected_text(format!("{:?}", self.provider)).show_ui(ui, |ui|{
                            ui.selectable_value(&mut self.provider, SearchProvider::Anilist, "Anilist");
                            ui.selectable_value(&mut self.provider, SearchProvider::AllAnime, "AllAnime");
                        
                    });
                ui.horizontal(|ui|{
                    
                    let search_bar = ui.add(egui::TextEdit::singleline(&mut self.anime).hint_text("Search for show..."));
                    
                    match self.provider{
                        SearchProvider::Anilist => {
                            if ui.button("Search").clicked() || (search_bar.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))){
                                if let Ok(pages) = search_anilist(&self.anime, &self.config){
                                    self.anilist_search = pages;
                                    
                                }
                            }
                            
                        },
                        SearchProvider::AllAnime => {
                        ComboBox::new("translation", "Translation Type").selected_text(format!("{:?}", self.translation)).show_ui(ui, |ui|{
                            ui.selectable_value(&mut self.translation, Translation::Sub, "Sub");
                            ui.selectable_value(&mut self.translation, Translation::Dub, "Dub");
                            ui.selectable_value(&mut self.translation, Translation::Raw, "Raw");
                        }
                        
                    );
                    if ui.button("Search").clicked() || (search_bar.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))){
                        if let Ok(shows) = get_shows(&self.anime, &self.translation, &self.config){
                            self.shows = shows.clone();
                        }  
                    }
                    
                        }
                    }
                    
                    
                });
                match self.provider{
                    SearchProvider::AllAnime =>{
                        if !self.shows.is_empty() {
                            egui::ScrollArea::vertical().id_salt("show_list").show(ui, |ui|{
                            let selected_states = vec![false; self.shows.clone().len()];

                            for (i,show) in self.shows.iter().enumerate()
                            {
                                let is_selected = selected_states[i];
                                ui.horizontal(|ui|{
                                    // ui.label(&show.id);
                                    // ui.separator();

                                    if ui.selectable_label(is_selected, &show.name).clicked(){
                                        self.selected_show = show.clone();
                                        if let Ok(eps) = get_episode_list(&show.id){
                                            self.episode_list = eps.clone();
                                        }
                                        self.previous_page = self.pages;
                                        self.pages = Pages::Show;
                                        ui.ctx().request_repaint();
                                        
                                    };
                                    ui.separator();
                                    ui.label("Number of episodes :");
                                    match self.translation{
                                        Translation::Sub => {
                                                if let Some(sub) = show.available_episodes.sub{
                                                    ui.label(sub.to_string());
                                                } else {
                                                    ui.label("No available episodes for that translation type");
                                                }
                                        },
                                        Translation::Dub => {
                                            if let Some(dub) = show.available_episodes.dub{
                                                ui.label(dub.to_string());
                                            } else {
                                                ui.label("No available episodes for that translation type");
                                            }
                                        },
                                        Translation::Raw => {
                                            if let Some(raw) = show.available_episodes.raw{
                                                ui.label(raw.to_string());
                                            } else {
                                                ui.label("No available episodes for that translation type");
                                            }
                                        },


                                    }
                                    
                                    
                                });
                                
                            }
                            });
                            
                        }

                    },
                    SearchProvider::Anilist => {
                        

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
                    }
                }
                
            },
            Pages::Show => {
                if self.panel_extended == true{
                    self.panel_extended = false;
                }
                ui.horizontal(|ui|{
                    if ui.selectable_label(self.pages.is_main_page(), "<").clicked(){
                        self.pages = self.previous_page;
                        ui.ctx().request_repaint();
                    }
                    ui.heading(&self.selected_show.name);

                    ComboBox::new("translation", "Translation Type").selected_text(format!("{:?}", self.translation)).show_ui(ui, |ui|{
                            ui.selectable_value(&mut self.translation, Translation::Sub, "Sub");
                            ui.selectable_value(&mut self.translation, Translation::Dub, "Dub");
                            ui.selectable_value(&mut self.translation, Translation::Raw, "Raw");
                        }
                    );
                    
                });
                if self.show_info == animeschedule::Anime::default(){
                    if let Ok(info) = get_next_episode_release(&self.selected_show){
                        self.show_info = info.clone();
                    }
                }        
                
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
                
                    
                self.play_translation(self.translation, ui);
                // match self.translation{
                //     Translation::Sub => {
                //         self.play_translation(self.translation, ui);
                //     },
                //     Translation::Dub => {
                //         if !self.episode_list.dub.is_empty(){
                //             let selected_states = vec![false; self.episode_list.dub.clone().len()];
                //             for (i,episodes) in self.episode_list.dub.iter().rev().enumerate(){
                //                 let mut is_selected = selected_states[i];
                //                 if let Some(ep) = episodes{
                //                     if ui.selectable_label(is_selected, format!("Episode: {ep}")).clicked(){
                //                         self.selected_episode = ep.clone();
                                        
                //                     }
                //                 }
                //             }
                //         }else {
                //             ui.label("No available episodes for selected translation.");
                //         }
                        
                        

                //     },
                //     Translation::Raw => {
                //         if !self.episode_list.raw.is_empty(){
                //             let selected_states = vec![false;self.episode_list.raw.clone().len()];
                //             for (i,episodes) in self.episode_list.raw.iter().rev().enumerate(){
                //                 let is_selected = selected_states[i];
                //                 if let Some(ep) = episodes{
                //                     if ui.selectable_label(is_selected, format!("Episode: {ep}")).clicked(){
                //                         self.selected_episode = ep.clone();
                //                         get_episode_url(&self.selected_show.id, &"raw".to_string(), ep);
                //                     }
                //                 }
                //             }
                //         }else {
                //             ui.label("No available episodes for selected translation.");
                //         }

                //     },
                // }

                // if !self.selected_episode.is_empty(){
                //     ui.label(format!("Now playing episode {} of {}", self.selected_episode, self.selected_show.name));
                // }
                
            },
            Pages::APIAuth =>{
                ui.heading("API Authorization");
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
                    
                    match check_credentials(&self.config.anilist.token){
                        Ok(id) => self.config.anilist.user_id = id,
                        Err(_) => {write_to_log("Unable to get User ID",ani_search::MessageType::Error); }
                    }
                    update_settings(&self.config);
                    self.previous_page = self.pages;
                    self.pages = Pages::Main;
                }

            },
            Pages::Settings => {
                ui.heading("AniSearch Settings");
                ui.separator();
                //dbg!(&self.config.app_settings);
                
                egui::Grid::new("settings")
                .num_columns(2)
                .spacing([40.0, 10.0])
                .show(ui, |ui| {
                    
                    ui.label(format!("Allow Adult"));
                    ui.add(egui::Checkbox::without_text(&mut self.config.app_settings.allow_adult));
                    ui.end_row();
                    
                    
                            
                        
                    });
                            


                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui|{
                    ui.horizontal(|ui|{
                        if ui.button("Save").clicked(){
                            update_settings(&self.config);
                            self.previous_page = self.pages;
                            self.pages = Pages::Main;
                        }
                        if ui.button("Apply").clicked(){
                            update_settings(&self.config);
                        }
                        
                    });
                    
                });
            },
            Pages::Anilist => {
                let user = &self.anilist_info.user;
                ui.horizontal(|ui|{
                    if ui.button("<").clicked(){
                        self.pages = self.previous_page;
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
            if ui.button("Test AniDB search").clicked(){
                if let Ok(client)=AniDbClient::new(){
                    tokio::spawn(async move{
                        let show = client.search("yani neko").await.expect("Unable to process query");
                        dbg!(&show);
                    });
                }
            }

        }

            
            
        });
    }
}

