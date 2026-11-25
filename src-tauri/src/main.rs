#![windows_subsystem = "windows"]

fn main() {
  if app_lib::try_handle_cli() {
    return;
  }
  app_lib::run();
}
