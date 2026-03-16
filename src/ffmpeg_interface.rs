extern crate subprocess;
use crate::utils::{is_snap, is_wayland};
use crate::wayland_record::{CursorModeTypes, RecordTypes, WaylandRecorder};
use chrono::prelude::*;
use filename::Filename;
use gettextrs::gettext;
use gtk::{prelude::*, ResponseType};
use gtk::{ButtonsType, DialogFlags, MessageDialog, MessageType};
use gtk::{CheckButton, ComboBoxText, Entry, FileChooserNative, SpinButton, Window};
use std::cell::RefCell;
use std::path::PathBuf;
use std::process::{Child, Command};
use std::rc::Rc;
use std::thread::sleep;
use std::time::Duration;
use subprocess::Exec;

#[derive(Clone)]
pub struct Ffmpeg {
    pub filename: (FileChooserNative, Entry, ComboBoxText),
    pub record_video: CheckButton,
    pub record_audio: CheckButton,
    pub audio_id: ComboBoxText,
    pub record_mouse: CheckButton,
    pub follow_mouse: CheckButton,
    pub record_frames: SpinButton,
    pub record_delay: SpinButton,
    pub command: Entry,
    pub video_process: Option<Rc<RefCell<Child>>>,
    pub audio_process: Option<Rc<RefCell<Child>>>,
    pub saved_filename: Option<String>,
    pub window: Window,
    pub record_wayland: WaylandRecorder,
    pub record_window: Rc<RefCell<bool>>,
    pub main_context: gtk::glib::MainContext,
    pub temp_video_filename: String,
}

impl Ffmpeg {
    pub fn start_record<F: FnOnce() + 'static>(
        this_rc: Rc<RefCell<Self>>,
        x: u16,
        y: u16,
        width: u16,
        height: u16,
        on_success: F,
    ) {
        let (main_context, window) = {
            let self_ = this_rc.borrow();
            (self_.main_context.clone(), self_.window.clone())
        };

        // Asignar el nombre de archivo primero
        {
            let mut self_ = this_rc.borrow_mut();
            self_.saved_filename = Some(
                self_.filename
                    .0
                    .file()
                    .unwrap()
                    .path()
                    .unwrap()
                    .join(PathBuf::from(format!(
                        "{}.{}",
                        if self_.filename.1.text().to_string().trim().eq("") {
                            Utc::now().to_string().replace(" UTC", "").replace(' ', "-")
                        } else {
                            self_.filename.1.text().to_string().trim().to_string()
                        },
                        self_.filename.2.active_id().unwrap()
                    )))
                    .as_path()
                    .display()
                    .to_string(),
            );
        }

        let is_file_already_exists =
            std::path::Path::new(&this_rc.borrow().saved_filename.clone().unwrap()).exists();

        // Lanzar UI dialog o iniciar grabación en spawn_local
        main_context.spawn_local(async move {
            if is_file_already_exists {
                let message_dialog = MessageDialog::new(
                    Some(&window),
                    DialogFlags::all(),
                    MessageType::Warning,
                    ButtonsType::YesNo,
                    &gettext("File already exist. Do you want to overwrite it?"),
                );

                let answer = message_dialog.run_future().await;
                message_dialog.close();

                if answer != ResponseType::Yes {
                    return;
                }
            }

            let is_wayland_active = is_wayland();
            let mut record_wayland_clone = None;

            {
                let mut self_ = this_rc.borrow_mut();

                if self_.record_video.is_active() && !is_wayland_active {
                    let mut ffmpeg_command: Command = Command::new("ffmpeg");

                    // record video with specified width and height
                    ffmpeg_command.args([
                        "-video_size",
                        format!("{}x{}", width, height).as_str(),
                        "-framerate",
                        self_.record_frames.value().to_string().as_str(),
                        "-f",
                        "x11grab",
                        "-i",
                        format!(
                            "{}+{},{}",
                            std::env::var("DISPLAY").unwrap_or_else(|_| ":0".to_string()).as_str(),
                            x,
                            y
                        )
                        .as_str(),
                    ]);

                    // if show mouse switch is enabled, draw the mouse to video
                    ffmpeg_command.arg("-draw_mouse");
                    if self_.record_mouse.is_active() {
                        ffmpeg_command.arg("1");
                    } else {
                        ffmpeg_command.arg("0");
                    }

                    // if follow mouse switch is enabled, follow the mouse
                    if self_.follow_mouse.is_active() {
                        ffmpeg_command.args(["-follow_mouse", "centered"]);
                    }

                    let video_filename = format!(
                        "{}.temp.without.audio.{}",
                        self_.saved_filename.as_ref().unwrap(),
                        self_.filename.2.active_id().unwrap()
                    );

                    ffmpeg_command.args([
                        "-crf",
                        "1",
                        if self_.record_audio.is_active() {
                            video_filename.as_str()
                        } else {
                            self_.saved_filename.as_ref().unwrap()
                        },
                        "-y",
                    ]);

                    // sleep for delay
                    sleep(Duration::from_secs(self_.record_delay.value() as u64));

                    // start recording and return the process id
                    self_.video_process = Some(Rc::new(RefCell::new(ffmpeg_command.spawn().unwrap())));
                } else if self_.record_video.is_active() && is_wayland_active {
                    sleep(Duration::from_secs(self_.record_delay.value() as u64));

                    let tempfile = tempfile::NamedTempFile::new()
                        .expect("cannot create temp file")
                        .keep()
                        .expect("cannot keep temp file");
                    self_.temp_video_filename = tempfile
                        .0
                        .file_name()
                        .expect("cannot get file name")
                        .to_str()
                        .unwrap()
                        .to_string();

                    let record_window = *self_.record_window.borrow();
                    let record_mouse_active = self_.record_mouse.is_active();

                    record_wayland_clone = Some((
                        self_.record_wayland.clone(),
                        self_.temp_video_filename.clone(),
                        record_window,
                        record_mouse_active,
                    ));
                }
            } // END borrow_mut for initialization

            // Async wait for Wayland portal, outside of the refcell borrow
            if let Some((mut record_wayland, temp_video_filename, record_window, record_mouse_active)) = record_wayland_clone {
                if !record_wayland.start(
                    temp_video_filename,
                    if record_window {
                        RecordTypes::Window
                    } else {
                        RecordTypes::Monitor
                    },
                    if record_mouse_active {
                        CursorModeTypes::Show
                    } else {
                        CursorModeTypes::Hidden
                    },
                ).await {
                    println!("failed to start recording");
                    return;
                }
                
                // Save mutated state back!
                this_rc.borrow_mut().record_wayland = record_wayland;
            }

            // Start audio if requested
            {
                let mut self_ = this_rc.borrow_mut();
                if self_.record_audio.is_active() {
                    let mut ffmpeg_command = Command::new("ffmpeg");
                    ffmpeg_command.arg("-f");
                    ffmpeg_command.arg("pulse");
                    ffmpeg_command.arg("-i");
                    ffmpeg_command.arg(self_.audio_id.active_id().unwrap());
                    ffmpeg_command.arg("-f");
                    ffmpeg_command.arg("ogg");
                    ffmpeg_command.arg(format!(
                        "{}.temp.audio",
                        self_.saved_filename.as_ref().unwrap()
                    ));
                    ffmpeg_command.arg("-y");
                    self_.audio_process = Some(Rc::new(RefCell::new(ffmpeg_command.spawn().unwrap())));
                }
            }

            // Finally trigger the UI callback saying everything started successfully
            on_success();
        });
    }

    pub fn stop_record<F: FnOnce() + 'static>(this_rc: Rc<RefCell<Self>>, on_success: F) {
        let (main_context, is_wayland_active) = {
            let self_ = this_rc.borrow();
            (self_.main_context.clone(), is_wayland())
        };

        main_context.spawn_local(async move {
            // First stop the processes
            {
                let self_ = this_rc.borrow_mut();

                // kill the process to stop recording
                if self_.video_process.is_some() {
                    let pid = self_.video_process.clone().unwrap().borrow_mut().id() as i32;
                    unsafe {
                        libc::kill(pid, libc::SIGTERM);
                    }

                    self_.video_process
                        .clone()
                        .unwrap()
                        .borrow_mut()
                        .wait()
                        .unwrap();

                    println!("video killed");
                }

                if self_.audio_process.is_some() {
                    let pid = self_.audio_process.clone().unwrap().borrow_mut().id() as i32;
                    unsafe {
                        libc::kill(pid, libc::SIGTERM);
                    }

                    self_.audio_process
                        .clone()
                        .unwrap()
                        .borrow_mut()
                        .wait()
                        .unwrap();
                    println!("audio killed");
                }
            } // END borrow_mut
            
            // Await wayland stop after dropping the RefMut borrow
            if is_wayland_active {
                // To avoid mutability issues while cloning we do it explicitly
                let mut record_wayland = {
                     this_rc.borrow().record_wayland.clone()
                };
                
                record_wayland.stop().await;
                
                // Write back mutated state
                this_rc.borrow_mut().record_wayland = record_wayland;
            }

            // Post processing like ffmpeg format conversions
            let (video_filename, audio_filename, saved_filename, file_format, command_text) = {
                let self_ = this_rc.borrow();
                let vf = if is_wayland_active {
                    self_.temp_video_filename.clone()
                } else {
                    format!(
                        "{}.temp.without.audio.{}",
                        self_.saved_filename.as_ref().unwrap(),
                        self_.filename.2.active_id().unwrap()
                    )
                };
                let af = format!("{}.temp.audio", self_.saved_filename.as_ref().unwrap());
                let sf = self_.saved_filename.clone().unwrap();
                let ff = self_.filename.2.active_id().unwrap().to_string();
                let cmd = self_.command.text().trim().to_string();
                (vf, af, sf, ff, cmd)
            };

            let is_video_record = std::path::Path::new(video_filename.as_str()).exists();
            let is_audio_record = std::path::Path::new(audio_filename.as_str()).exists();

            if is_video_record {
                if is_wayland_active {
                    // convert webm to specified format
                    Command::new("ffmpeg")
                        .args([
                            "-i",
                            video_filename.as_str(),
                            "-crf",
                            "23", // default quality
                            "-c:a",
                            file_format.as_str(),
                            saved_filename.as_str(),
                            "-y",
                        ])
                        .output()
                        .unwrap();
                } else {
                    let mut move_command = Command::new("mv");
                    move_command.args([
                        saved_filename.as_str(),
                        if is_audio_record {
                            video_filename.as_str()
                        } else {
                            saved_filename.as_str()
                        },
                    ]);
                    move_command.output().unwrap();
                }

                // if audio record, then merge video and audio
                if is_audio_record {
                    Command::new("ffmpeg")
                        .args([
                            "-i",
                            video_filename.as_str(),
                            "-f",
                            "ogg",
                            "-i",
                            audio_filename.as_str(),
                            "-crf",
                            "23", // default quality
                            "-c:a",
                            "aac",
                            saved_filename.as_str(),
                            "-y",
                        ])
                        .output()
                        .expect("failed to merge video and audio");

                    std::fs::remove_file(audio_filename).unwrap();
                }

                std::fs::remove_file(video_filename).unwrap();
            }
            // if only audio is recording then convert it to chosen format
            else if is_audio_record {
                Command::new("ffmpeg")
                    .args([
                        "-f",
                        "ogg",
                        "-i",
                        audio_filename.as_str(),
                        saved_filename.as_str(),
                    ])
                    .output()
                    .expect("failed convert audio to video");

                std::fs::remove_file(audio_filename).unwrap();
            }

            // execute command after finish recording
            if command_text != "" {
                Exec::shell(command_text.as_str()).popen().unwrap();
            }

            on_success();
        });
    }

    pub fn play_record(self) {
        if self.saved_filename.is_some() {
            if is_snap() {
                // open the video using snapctrl for snap package
                Command::new("snapctl")
                    .arg("user-open")
                    .arg(self.saved_filename.unwrap())
                    .spawn()
                    .unwrap();
            } else {
                Command::new("xdg-open")
                    .arg(self.saved_filename.unwrap())
                    .spawn()
                    .unwrap();
            }
        }
    }
}
