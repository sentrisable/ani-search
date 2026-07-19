use std::{time::Duration, sync::{mpsc::{Receiver, Sender}, Arc, Mutex}, task, process};

use eframe::egui;

use egui::{ComboBox};
use tokio::runtime::Runtime;


use ani_search::{
    AvailableEpisodes, Edge, Translation, animeschedule, episode_source, episodes, generate_link, get_episode_list, get_episode_url, get_next_episode_release, get_shows,

    
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

enum Pages{
    Main,
    Show,
}

impl Pages{
    fn is_main_page(&self) -> bool {
        matches!(self, Pages::Main)
    }
    fn is_show_page(&self) -> bool {
        matches!(self, Pages::Show)
    }

}



struct Main{
    pages: Pages,
    anime: String,
    translation: Translation,
    shows: Vec<Edge>,
    episode_list: episodes::AvailableEpisodesDetail,
    show_info: animeschedule::Anime,
    selected_show: Edge,
    selected_episode: String,
    selected_episode_urls: Vec<(String,Vec<(i32,String)>)>,
    //selected_episode_urls: Vec<episode_source::SourceUrl>,
    tx: Sender<String>,
    rx: Receiver<String>,
}

impl Default for Main{
    fn default() -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        Self {
            pages: Pages::Main,
            anime: Default::default(),
            translation: Translation::Sub,
            shows: Vec::new(),
            show_info: animeschedule::Anime::default(),
            selected_show: Edge::default(),
            selected_episode: Default::default(),
            selected_episode_urls: Vec::new(),

            episode_list: episodes::AvailableEpisodesDetail::default(),
            tx, 
            rx 
        }
    }
}

impl Main{
    fn new(cc: &eframe::CreationContext<'_>)->Self{
        Self::default()
    }
}

impl eframe::App for Main{
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame){
        egui::CentralPanel::default().show(ui, |ui| match self.pages{
            Pages::Main => {
                ui.horizontal(|ui|{
                    ui.add(egui::TextEdit::singleline(&mut self.anime).hint_text("Search for show..."));
                    
                    ComboBox::new("translation", "Translation Type").selected_text(format!("{:?}", self.translation)).show_ui(ui, |ui|{
                            ui.selectable_value(&mut self.translation, Translation::Sub, "Sub");
                            ui.selectable_value(&mut self.translation, Translation::Dub, "Dub");
                            ui.selectable_value(&mut self.translation, Translation::Raw, "Raw");
                        }
                    );
                    
                    if ui.button("Search").clicked(){
                        if let Ok(shows) = get_shows(&self.anime, &self.translation){
                            self.shows = shows.clone();
                        }
                    }
                });

                if !self.shows.is_empty(){
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
            Pages::Show => {
                ui.horizontal(|ui|{
                    if ui.selectable_label(self.pages.is_main_page(), "<").clicked(){
                        self.pages = Pages::Main;
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
                
                    
                
                match self.translation{
                    Translation::Sub => {
                        if !self.episode_list.sub.is_empty(){
                            ui.heading("Subbed Episodes");
                            let selected_states = vec![false;self.episode_list.sub.clone().len()];
                            for (i,episodes) in self.episode_list.sub.iter().rev().enumerate(){
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
                                        ui.horizontal(|ui|{
                                            ui.label("Sources: ");
                                            let selected_url = vec![false; self.selected_episode_urls.clone().len()];
                                            for (i,url) in self.selected_episode_urls.clone().iter().enumerate(){
                                                let source_name = &url.0;
                                                    match source_name.as_str(){
                                                        // will add later once I get them set up
                                                        "Mp4"| "Fm-Hls" | "S-mp4" | "Yt-mp4" => {
                                                            
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
                                                                    let spawn_player = process::Command::new("mpv").arg("--tls-verify=no").arg(media_title_arg).arg(refer_flag_arg).arg(episode_link).spawn();
                                                                
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
                        } else {
                            ui.label("No available episodes for selected translation.");
                        }
                        

                    },
                    Translation::Dub => {
                        if !self.episode_list.raw.is_empty(){
                            let selected_states = vec![false; self.episode_list.dub.clone().len()];
                            for (i,episodes) in self.episode_list.dub.iter().rev().enumerate(){
                                let mut is_selected = selected_states[i];
                                if let Some(ep) = episodes{
                                    if ui.selectable_label(is_selected, format!("Episode: {ep}")).clicked(){
                                        self.selected_episode = ep.clone();
                                        
                                    }
                                }
                            }
                        }else {
                            ui.label("No available episodes for selected translation.");
                        }
                        
                        

                    },
                    Translation::Raw => {
                        if !self.episode_list.raw.is_empty(){
                            let selected_states = vec![false;self.episode_list.raw.clone().len()];
                            for (i,episodes) in self.episode_list.raw.iter().rev().enumerate(){
                                let is_selected = selected_states[i];
                                if let Some(ep) = episodes{
                                    if ui.selectable_label(is_selected, format!("Episode: {ep}")).clicked(){
                                        self.selected_episode = ep.clone();
                                        get_episode_url(&self.selected_show.id, &"raw".to_string(), ep);
                                    }
                                }
                            }
                        }else {
                            ui.label("No available episodes for selected translation.");
                        }

                    },
                }

                // if !self.selected_episode.is_empty(){
                //     ui.label(format!("Now playing episode {} of {}", self.selected_episode, self.selected_show.name));
                // }
                
            }
            
            
        });
    }
}
