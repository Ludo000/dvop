mod ui;
mod handlers;
mod utils;

use gtk4::prelude::*;
use gtk4::Application;

fn main() {
    let app = Application::builder()
        .application_id("com.example.BasadoTextEditor")
        .build();

    app.connect_activate(build_ui);
    app.run();
}

fn build_ui(app: &Application) {
    let window = ui::create_window(app);
    let (header, new_button, open_button, save_button, save_as_button) = ui::create_header();
    let (text_view, text_buffer, file_path, error_label, picture, current_dir, scrolled_window) = ui::create_text_view();
    let terminal = ui::create_terminal();
    let terminal_box = ui::create_terminal_box(&terminal);
    let (file_list_box, file_list_scrolled_window, nav_box, up_button, refresh_button) = ui::create_file_manager_panel();
    let file_manager_panel = ui::create_file_manager_panel_container(nav_box, file_list_scrolled_window);
    let paned = ui::create_paned(&file_manager_panel, &scrolled_window, &terminal_box);

    window.set_titlebar(Some(&header));

    utils::update_file_list(&file_list_box, &current_dir.borrow(), &file_path.borrow());

    window.set_child(Some(&paned));

    handlers::setup_button_handlers(
        &new_button, &open_button, &save_button, &save_as_button,
        &text_buffer, &file_path, &window, &current_dir, &file_list_box,
        &scrolled_window, &text_view, &error_label, &picture,
        &up_button, &refresh_button, &file_list_box
    );

    window.show();
}
