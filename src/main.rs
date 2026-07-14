use std::{time::Duration, sync::{mpsc::{Receiver, Sender}, Arc, Mutex}, task};

use eframe::egui;

use egui::{ComboBox};
use tokio::runtime::Runtime;


use ani_search::{
    Edge, Translation, episodes, get_episode_list, get_shows,

    
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



struct Main{
    anime: String,
    translation: Translation,
    shows: Vec<Edge>,
    episode_list: episodes::AvailableEpisodesDetail,
    tx: Sender<String>,
    rx: Receiver<String>,
}

impl Default for Main{
    fn default() -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        Self {
            anime: Default::default(),
            translation: Translation::Sub,
            shows: Vec::new(),
            episode_list:  episodes::AvailableEpisodesDetail { dub: Vec::new(), raw: Vec::new(), sub:  Vec::new() },
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
        egui::CentralPanel::default().show(ui, |ui|{
            ui.horizontal(|ui|{
                ui.add(egui::TextEdit::singleline(&mut self.anime).hint_text("Search for show..."));
                ComboBox::new("translation", "Translation Type").selected_text(format!("{:?}", self.translation)).show_ui(ui, |ui|{
                        ui.selectable_value(&mut self.translation, Translation::Sub, "Sub");
                        ui.selectable_value(&mut self.translation, Translation::Dub, "Dub");
                        ui.selectable_value(&mut self.translation, Translation::Raw, "Raw");
                    }
                );
                
                if ui.button("Search").clicked(){
                    if let Ok(shows) = get_shows(){
                        self.shows = shows.clone();
                    }
                }
            });

            if !self.shows.is_empty(){
                let selected_states = vec![false; self.shows.clone().len()];
                for (i,show) in self.shows.iter().enumerate()
                {
                    let is_selected = selected_states[i];
                    ui.horizontal(|ui|{
                        // ui.label(&show.id);
                        // ui.separator();

                        if ui.selectable_label(is_selected, &show.name).clicked(){
                            if let Ok(eps) = get_episode_list(&show.id){
                                self.episode_list = eps.clone();
                            }
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
            }

            
        });
    }
}
