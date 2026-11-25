use chrono::{Local, NaiveTime, TimeDelta};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PowerAction {
    Shutdown,
    Restart,
    Sleep,
    Lock,
    Logout,
}

impl Default for PowerAction {
    fn default() -> Self {
        Self::Shutdown
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ShutdownTimerStatus {
    pub active: bool,
    pub remaining_seconds: u64,
    pub action: PowerAction,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StartPowerTimerRequest {
    pub action: PowerAction,
    pub mode: String, // "timespan" | "time_of_day"
    pub hours: Option<u64>,
    pub minutes: Option<u64>,
    pub seconds: Option<u64>,
    pub target_time: Option<String>, // "HH:MM" (24h)
    pub graceful: bool,
    pub prevent_sleep: bool,
}

struct TimerInternal {
    deadline: Option<Instant>,
    generation: u64,
    action: PowerAction,
}

pub struct ShutdownTimerManager {
    inner: Arc<Mutex<TimerInternal>>,
}

impl ShutdownTimerManager {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(TimerInternal {
                deadline: None,
                generation: 0,
                action: PowerAction::Shutdown,
            })),
        }
    }

    pub fn get_status(&self) -> ShutdownTimerStatus {
        let guard = self.inner.lock();
        if let Some(deadline) = guard.deadline {
            let now = Instant::now();
            if deadline > now {
                return ShutdownTimerStatus {
                    active: true,
                    remaining_seconds: (deadline - now).as_secs(),
                    action: guard.action.clone(),
                };
            }
        }
        ShutdownTimerStatus {
            active: false,
            remaining_seconds: 0,
            action: guard.action.clone(),
        }
    }

    pub fn start(&self, app: AppHandle, req: StartPowerTimerRequest) -> Result<ShutdownTimerStatus, String> {
        let total_secs: u64 = if req.mode == "time_of_day" {
            let time_str = req
                .target_time
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .ok_or_else(|| "Please choose a target time of day.".to_string())?;

            let parsed = NaiveTime::parse_from_str(time_str, "%H:%M")
                .or_else(|_| NaiveTime::parse_from_str(time_str, "%H:%M:%S"))
                .map_err(|_| "Invalid time format. Use HH:MM.".to_string())?;

            let now = Local::now();
            let today_target = now
                .date_naive()
                .and_time(parsed)
                .and_local_timezone(Local)
                .single()
                .ok_or_else(|| "Could not resolve local target time.".to_string())?;

            let target_dt = if today_target <= now {
                today_target + TimeDelta::days(1)
            } else {
                today_target
            };

            let diff = (target_dt - now).num_seconds();
            if diff <= 0 {
                1
            } else {
                diff as u64
            }
        } else {
            let h = req.hours.unwrap_or(0);
            let m = req.minutes.unwrap_or(0);
            let s = req.seconds.unwrap_or(0);
            let total = h
                .saturating_mul(3600)
                .saturating_add(m.saturating_mul(60))
                .saturating_add(s);

            if total == 0 {
                return Err("Please set a duration greater than 0 seconds.".to_string());
            }
            if total > 7 * 86400 {
                return Err("Maximum timer duration is 7 days.".to_string());
            }
            total
        };

        let deadline = Instant::now() + Duration::from_secs(total_secs);
        let action = req.action.clone();
        let graceful = req.graceful;
        let prevent_sleep = req.prevent_sleep;

        let generation = {
            let mut guard = self.inner.lock();
            guard.generation = guard.generation.wrapping_add(1);
            guard.deadline = Some(deadline);
            guard.action = action.clone();
            guard.generation
        };

        let status = ShutdownTimerStatus {
            active: true,
            remaining_seconds: total_secs,
            action: action.clone(),
        };
        let _ = app.emit("shutdown-timer-tick", &status);

        let inner_clone = Arc::clone(&self.inner);
        std::thread::spawn(move || {
            #[cfg(windows)]
            if prevent_sleep {
                use windows_sys::Win32::System::Power::{
                    SetThreadExecutionState, ES_CONTINUOUS, ES_SYSTEM_REQUIRED,
                };
                unsafe {
                    SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED);
                }
            }

            loop {
                std::thread::sleep(Duration::from_secs(1));

                let mut should_execute = false;
                let tick_status: Option<ShutdownTimerStatus>;

                {
                    let mut guard = inner_clone.lock();
                    if guard.generation != generation {
                        break;
                    }
                    if let Some(dl) = guard.deadline {
                        let now = Instant::now();
                        if now >= dl {
                            guard.deadline = None;
                            should_execute = true;
                            tick_status = Some(ShutdownTimerStatus {
                                active: false,
                                remaining_seconds: 0,
                                action: action.clone(),
                            });
                        } else {
                            let rem = (dl - now).as_secs();
                            tick_status = Some(ShutdownTimerStatus {
                                active: true,
                                remaining_seconds: rem,
                                action: action.clone(),
                            });
                        }
                    } else {
                        break;
                    }
                }

                if let Some(st) = tick_status {
                    let _ = app.emit("shutdown-timer-tick", &st);
                }

                if should_execute {
                    #[cfg(windows)]
                    if prevent_sleep {
                        use windows_sys::Win32::System::Power::{
                            SetThreadExecutionState, ES_CONTINUOUS,
                        };
                        unsafe {
                            SetThreadExecutionState(ES_CONTINUOUS);
                        }
                    }
                    execute_power_action(&action, graceful);
                    return;
                }
            }

            #[cfg(windows)]
            if prevent_sleep {
                use windows_sys::Win32::System::Power::{SetThreadExecutionState, ES_CONTINUOUS};
                unsafe {
                    SetThreadExecutionState(ES_CONTINUOUS);
                }
            }
        });

        Ok(status)
    }

    pub fn cancel(&self, app: AppHandle) -> ShutdownTimerStatus {
        let action = {
            let mut guard = self.inner.lock();
            guard.generation = guard.generation.wrapping_add(1);
            guard.deadline = None;
            guard.action.clone()
        };

        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            let _ = std::process::Command::new("shutdown")
                .args(["/a"])
                .creation_flags(CREATE_NO_WINDOW)
                .spawn();
        }

        let status = ShutdownTimerStatus {
            active: false,
            remaining_seconds: 0,
            action,
        };
        let _ = app.emit("shutdown-timer-tick", &status);
        status
    }
}

fn execute_power_action(action: &PowerAction, graceful: bool) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        match action {
            PowerAction::Shutdown => {
                let mut args = vec!["/s", "/t", "0"];
                if !graceful {
                    args.push("/f");
                }
                let _ = std::process::Command::new("shutdown")
                    .args(&args)
                    .creation_flags(CREATE_NO_WINDOW)
                    .spawn();
            }
            PowerAction::Restart => {
                let mut args = vec!["/r", "/t", "0"];
                if !graceful {
                    args.push("/f");
                }
                let _ = std::process::Command::new("shutdown")
                    .args(&args)
                    .creation_flags(CREATE_NO_WINDOW)
                    .spawn();
            }
            PowerAction::Sleep => {
                let _ = std::process::Command::new("rundll32.exe")
                    .args(["powrprof.dll,SetSuspendState", "0,1,0"])
                    .creation_flags(CREATE_NO_WINDOW)
                    .spawn();
            }
            PowerAction::Lock => {
                let _ = std::process::Command::new("rundll32.exe")
                    .args(["user32.dll,LockWorkStation"])
                    .creation_flags(CREATE_NO_WINDOW)
                    .spawn();
            }
            PowerAction::Logout => {
                let _ = std::process::Command::new("shutdown")
                    .args(["/l"])
                    .creation_flags(CREATE_NO_WINDOW)
                    .spawn();
            }
        }
    }

    #[cfg(not(windows))]
    {
        let _ = (action, graceful);
    }
}
