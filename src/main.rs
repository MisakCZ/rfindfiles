use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use gtk::gdk;
use gtk::gio;
use gtk::prelude::*;
use gtk::{
    Align, Application, ApplicationWindow, Box as GtkBox, Button, Calendar, CheckButton,
    CssProvider, Entry, FileChooserAction, FileChooserNative, GestureClick, HeaderBar, Label,
    ListBox, ListBoxRow, MenuButton, Orientation, Popover, ResponseType, ScrolledWindow,
};
use rfindfiles::{CliError, Config, find_files, parse_date, parse_extensions, parse_folders};

const RESULTS_PER_PAGE: usize = 50;
const DATE_BUTTON_LABEL: &str = "Vybrat datum";
const CUSTOM_THEME_FILE: &str = "theme.txt";
const COMMON_EXTENSIONS: [&str; 6] = ["pdf", "docx", "xlsx", "txt", "jpg", "png"];
const THEMES: [ThemeDefinition; 4] = [
    ThemeDefinition {
        id: "theme-sand",
        label: "Písek",
    },
    ThemeDefinition {
        id: "theme-forest",
        label: "Les",
    },
    ThemeDefinition {
        id: "theme-night",
        label: "Noc",
    },
    ThemeDefinition {
        id: "theme-ink",
        label: "Inkoust",
    },
];
const DEFAULT_THEME_ID: &str = "theme-sand";
const APP_CSS: &str = r#"
window.theme-sand {
    background: linear-gradient(180deg, #f5efe3 0%, #e6d7bc 100%);
    color: #2f2618;
}

window.theme-sand entry,
window.theme-sand button,
window.theme-sand menubutton > button,
window.theme-sand listbox,
window.theme-sand scrolledwindow,
window.theme-sand popover,
window.theme-sand calendar {
    background: #fff9ef;
    color: #2f2618;
    border-color: #cdb48a;
}

window.theme-sand .accent-button {
    background: #b66d39;
    color: #fffaf4;
}

window.theme-forest {
    background: linear-gradient(180deg, #dce8dc 0%, #c1d5c1 100%);
    color: #17321f;
}

window.theme-forest entry,
window.theme-forest button,
window.theme-forest menubutton > button,
window.theme-forest listbox,
window.theme-forest scrolledwindow,
window.theme-forest popover,
window.theme-forest calendar {
    background: #f0f7ef;
    color: #17321f;
    border-color: #82a089;
}

window.theme-forest .accent-button {
    background: #2f6943;
    color: #f5fbf5;
}

window.theme-night {
    background: linear-gradient(180deg, #1d2433 0%, #101723 100%);
    color: #e5edf8;
}

window.theme-night entry,
window.theme-night button,
window.theme-night menubutton > button,
window.theme-night listbox,
window.theme-night scrolledwindow,
window.theme-night popover,
window.theme-night calendar {
    background: #243046;
    color: #e5edf8;
    border-color: #5b77a5;
}

window.theme-night .accent-button {
    background: #77afff;
    color: #101723;
}

window.theme-ink {
    background: linear-gradient(180deg, #171717 0%, #0b0f18 100%);
    color: #f1efe9;
}

window.theme-ink entry,
window.theme-ink button,
window.theme-ink menubutton > button,
window.theme-ink listbox,
window.theme-ink scrolledwindow,
window.theme-ink popover,
window.theme-ink calendar {
    background: #1e2430;
    color: #f1efe9;
    border-color: #8f9bb0;
}

window.theme-ink .accent-button {
    background: #d9ba72;
    color: #141414;
}

.result-row {
    padding: 8px 10px;
}
"#;

#[derive(Clone, Copy)]
struct ThemeDefinition {
    id: &'static str,
    label: &'static str,
}

#[derive(Default)]
struct PaginationState {
    all_results: Vec<PathBuf>,
    current_page: usize,
}

impl PaginationState {
    fn set_results(&mut self, results: Vec<PathBuf>) {
        self.all_results = results;
        self.current_page = 0;
    }

    fn total_pages(&self) -> usize {
        if self.all_results.is_empty() {
            0
        } else {
            self.all_results.len().div_ceil(RESULTS_PER_PAGE)
        }
    }

    fn visible_items(&self) -> &[PathBuf] {
        if self.all_results.is_empty() {
            return &[];
        }

        let start = self.current_page * RESULTS_PER_PAGE;
        let end = usize::min(start + RESULTS_PER_PAGE, self.all_results.len());
        &self.all_results[start..end]
    }

    fn can_go_previous(&self) -> bool {
        self.current_page > 0
    }

    fn can_go_next(&self) -> bool {
        self.current_page + 1 < self.total_pages()
    }
}

fn main() {
    let app = Application::builder()
        .application_id("com.example.rfindfiles")
        .build();

    app.connect_activate(build_ui);
    app.run();
}

fn build_ui(app: &Application) {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Rust File Finder")
        .default_width(980)
        .default_height(700)
        .build();

    setup_css(&window);

    let root = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(12)
        .margin_top(16)
        .margin_bottom(16)
        .margin_start(16)
        .margin_end(16)
        .build();

    let headerbar = HeaderBar::new();
    headerbar.set_title_widget(Some(&Label::new(Some("Rust File Finder"))));
    let theme_menu = build_theme_menu(&window);
    headerbar.pack_end(&theme_menu);
    window.set_titlebar(Some(&headerbar));
    apply_theme(&window, &load_saved_theme());

    let folders_entry = Entry::builder()
        .hexpand(true)
        .placeholder_text("Složky oddělené čárkou")
        .build();

    let add_folder_button = Button::with_label("Přidat složku");
    let search_button = Button::with_label("Hledat");
    search_button.add_css_class("accent-button");
    let previous_button = Button::with_label("Předchozí");
    let next_button = Button::with_label("Další");
    let page_label = Label::new(Some("Strana 0 / 0"));
    let status_label = Label::new(None);
    status_label.set_xalign(0.0);

    let extensions_box = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(12)
        .build();
    let extension_checkboxes = build_extension_checkboxes(&extensions_box);
    let custom_extensions_entry = Entry::builder()
        .hexpand(true)
        .placeholder_text("Vlastní přípony, např. md,csv,json")
        .build();
    extensions_box.append(&Label::new(Some("Vlastní:")));
    extensions_box.append(&custom_extensions_entry);

    let (date_from_widget, date_from_button) = build_date_picker("Od");
    let (date_to_widget, date_to_button) = build_date_picker("Do");

    let folders_row = build_row(
        "Složky",
        &[
            folders_entry.clone().upcast::<gtk::Widget>(),
            add_folder_button.clone().upcast::<gtk::Widget>(),
        ],
    );
    let filters_row = build_row("Přípony", &[extensions_box.clone().upcast::<gtk::Widget>()]);
    let dates_row = build_row(
        "Datum",
        &[
            date_from_widget.upcast::<gtk::Widget>(),
            date_to_widget.upcast::<gtk::Widget>(),
            search_button.clone().upcast::<gtk::Widget>(),
        ],
    );

    let results_list = ListBox::new();
    results_list.set_vexpand(true);
    results_list.set_activate_on_single_click(false);

    let scrolled = ScrolledWindow::builder()
        .hexpand(true)
        .vexpand(true)
        .min_content_height(320)
        .child(&results_list)
        .build();

    let pagination_row = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(8)
        .build();
    pagination_row.append(&previous_button);
    pagination_row.append(&next_button);
    pagination_row.append(&page_label);

    root.append(&folders_row);
    root.append(&filters_row);
    root.append(&dates_row);
    root.append(&status_label);
    root.append(&scrolled);
    root.append(&pagination_row);

    let state = Rc::new(RefCell::new(PaginationState::default()));

    {
        let window = window.clone();
        let folders_entry = folders_entry.clone();
        let status_label = status_label.clone();
        add_folder_button.connect_clicked(move |_| {
            let dialog = FileChooserNative::new(
                Some("Vyberte složku"),
                Some(&window),
                FileChooserAction::SelectFolder,
                Some("Vybrat"),
                Some("Zrušit"),
            );

            let folders_entry = folders_entry.clone();
            let status_label = status_label.clone();
            dialog.connect_response(move |dialog, response| {
                if response == ResponseType::Accept {
                    if let Some(file) = dialog.file() {
                        if let Some(path) = file.path() {
                            append_folder(&folders_entry, &path);
                            status_label.set_text("");
                        }
                    }
                }
                dialog.destroy();
            });

            dialog.show();
        });
    }

    {
        let results_list = results_list.clone();
        let page_label = page_label.clone();
        let previous_button = previous_button.clone();
        let next_button = next_button.clone();
        let status_label = status_label.clone();
        let folders_entry = folders_entry.clone();
        let custom_extensions_entry = custom_extensions_entry.clone();
        let date_from_button = date_from_button.clone();
        let date_to_button = date_to_button.clone();
        let extension_checkboxes = extension_checkboxes.clone();
        let state = state.clone();
        search_button.connect_clicked(move |_| {
            let extensions_raw =
                collect_extensions(&extension_checkboxes, &custom_extensions_entry);
            match build_config(
                &folders_entry.text(),
                &extensions_raw,
                &button_date_value(&date_from_button),
                &button_date_value(&date_to_button),
            ) {
                Ok(config) => {
                    let results = find_files(&config);
                    let total = results.len();
                    state.borrow_mut().set_results(results);
                    refresh_results(
                        &results_list,
                        &page_label,
                        &previous_button,
                        &next_button,
                        &state,
                    );
                    status_label.set_text(&format!("Nalezeno souborů: {total}"));
                }
                Err(err) => {
                    state.borrow_mut().set_results(Vec::new());
                    refresh_results(
                        &results_list,
                        &page_label,
                        &previous_button,
                        &next_button,
                        &state,
                    );
                    status_label.set_text(&err.to_string());
                }
            }
        });
    }

    {
        let results_list = results_list.clone();
        let page_label = page_label.clone();
        let previous_button = previous_button.clone();
        let next_button = next_button.clone();
        let state_handle = state.clone();
        let previous_button_for_refresh = previous_button.clone();
        let next_button_for_refresh = next_button.clone();
        previous_button.connect_clicked(move |_| {
            let mut state = state_handle.borrow_mut();
            if state.can_go_previous() {
                state.current_page -= 1;
            }
            drop(state);
            refresh_results(
                &results_list,
                &page_label,
                &previous_button_for_refresh,
                &next_button_for_refresh,
                &state_handle,
            );
        });
    }

    {
        let results_list = results_list.clone();
        let page_label = page_label.clone();
        let previous_button = previous_button.clone();
        let next_button = next_button.clone();
        let state_handle = state.clone();
        let previous_button_for_refresh = previous_button.clone();
        let next_button_for_refresh = next_button.clone();
        next_button.connect_clicked(move |_| {
            let mut state = state_handle.borrow_mut();
            if state.can_go_next() {
                state.current_page += 1;
            }
            drop(state);
            refresh_results(
                &results_list,
                &page_label,
                &previous_button_for_refresh,
                &next_button_for_refresh,
                &state_handle,
            );
        });
    }

    refresh_results(
        &results_list,
        &page_label,
        &previous_button,
        &next_button,
        &state,
    );

    window.set_child(Some(&root));
    window.present();
}

fn setup_css(window: &ApplicationWindow) {
    let provider = CssProvider::new();
    provider.load_from_data(APP_CSS);
    if let Some(display) = gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    } else {
        let display = gtk::prelude::WidgetExt::display(window);
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

fn build_theme_menu(window: &ApplicationWindow) -> MenuButton {
    let menu_button = MenuButton::builder().label("Vzhled").build();
    let popover = Popover::new();
    let container = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(6)
        .margin_top(8)
        .margin_bottom(8)
        .margin_start(8)
        .margin_end(8)
        .build();

    for theme in THEMES {
        let button = Button::with_label(theme.label);
        let window = window.clone();
        let popover = popover.clone();
        button.connect_clicked(move |_| {
            apply_theme(&window, theme.id);
            let _ = save_theme(theme.id);
            popover.popdown();
        });
        container.append(&button);
    }

    popover.set_child(Some(&container));
    menu_button.set_popover(Some(&popover));
    menu_button
}

fn build_extension_checkboxes(container: &GtkBox) -> Vec<(String, CheckButton)> {
    let mut result = Vec::with_capacity(COMMON_EXTENSIONS.len());

    for extension in COMMON_EXTENSIONS {
        let checkbox = CheckButton::with_label(extension);
        container.append(&checkbox);
        result.push((extension.to_string(), checkbox));
    }

    result
}

fn build_date_picker(prefix: &str) -> (GtkBox, MenuButton) {
    let wrapper = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(6)
        .build();
    let label = Label::new(Some(prefix));
    let button = MenuButton::builder().label(DATE_BUTTON_LABEL).build();
    let popover = Popover::new();
    let calendar = Calendar::new();
    let clear_button = Button::with_label("Vymazat");

    {
        let button = button.clone();
        let popover = popover.clone();
        calendar.connect_day_selected(move |calendar| {
            let date = calendar.date();
            let formatted = format!(
                "{:04}-{:02}-{:02}",
                date.year(),
                date.month(),
                date.day_of_month()
            );
            button.set_label(&formatted);
            popover.popdown();
        });
    }

    {
        let button = button.clone();
        clear_button.connect_clicked(move |_| {
            button.set_label(DATE_BUTTON_LABEL);
        });
    }

    popover.set_child(Some(&calendar));
    button.set_popover(Some(&popover));
    wrapper.append(&label);
    wrapper.append(&button);
    wrapper.append(&clear_button);

    (wrapper, button)
}

fn build_row(label_text: &str, widgets: &[gtk::Widget]) -> GtkBox {
    let row = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(8)
        .build();

    let label = Label::new(Some(label_text));
    label.set_width_chars(10);
    label.set_xalign(0.0);
    label.set_halign(Align::Start);
    row.append(&label);

    for widget in widgets {
        row.append(widget);
    }

    row
}

fn append_folder(entry: &Entry, path: &Path) {
    let current = entry.text().to_string();
    let incoming = path.display().to_string();

    if current
        .split(',')
        .map(|item| item.trim())
        .any(|item| item == incoming)
    {
        return;
    }

    if current.trim().is_empty() {
        entry.set_text(&incoming);
    } else {
        entry.set_text(&format!("{current}, {incoming}"));
    }
}

fn collect_extensions(checkboxes: &[(String, CheckButton)], custom_entry: &Entry) -> String {
    let mut selected = Vec::new();

    for (extension, checkbox) in checkboxes {
        if checkbox.is_active() {
            selected.push(extension.clone());
        }
    }

    let custom = custom_entry.text();
    if !custom.trim().is_empty() {
        selected.push(custom.to_string());
    }

    selected.join(",")
}

fn button_date_value(button: &MenuButton) -> String {
    match button.label() {
        Some(label) if label != DATE_BUTTON_LABEL => label.to_string(),
        _ => String::new(),
    }
}

fn build_config(
    folders_raw: &str,
    extensions_raw: &str,
    date_from_raw: &str,
    date_to_raw: &str,
) -> Result<Config, CliError> {
    let folders = parse_folders(folders_raw);
    if folders.is_empty() {
        return Err(CliError::Message(
            "Vyberte alespoň jednu existující složku.".to_string(),
        ));
    }

    let extensions = parse_extensions(extensions_raw);
    if extensions.is_empty() {
        return Err(CliError::Message(
            "Vyberte alespoň jeden typ souboru.".to_string(),
        ));
    }

    let date_from = if date_from_raw.trim().is_empty() {
        None
    } else {
        Some(parse_date(date_from_raw, false)?)
    };
    let date_to = if date_to_raw.trim().is_empty() {
        None
    } else {
        Some(parse_date(date_to_raw, true)?)
    };

    if let (Some(from), Some(to)) = (date_from, date_to) {
        if from > to {
            return Err(CliError::Message(
                "Pole 'Datum od' musí být menší nebo rovno 'Datum do'.".to_string(),
            ));
        }
    }

    Ok(Config {
        folders,
        extensions,
        date_from,
        date_to,
    })
}

fn refresh_results(
    results_list: &ListBox,
    page_label: &Label,
    previous_button: &Button,
    next_button: &Button,
    state: &Rc<RefCell<PaginationState>>,
) {
    while let Some(child) = results_list.first_child() {
        results_list.remove(&child);
    }

    let state_ref = state.borrow();
    for path in state_ref.visible_items() {
        let label = Label::new(Some(&path.display().to_string()));
        label.set_xalign(0.0);
        label.add_css_class("result-row");
        label.set_tooltip_text(Some(&path.display().to_string()));

        let row = ListBoxRow::new();
        row.set_child(Some(&label));
        row.set_activatable(true);
        row.set_selectable(true);

        let path = path.clone();
        let gesture = GestureClick::new();
        gesture.set_button(1);
        gesture.connect_pressed(move |_gesture, n_press, _x, _y| {
            if n_press == 2 {
                open_path(&path);
            }
        });
        row.add_controller(gesture);

        results_list.append(&row);
    }

    let total_pages = state_ref.total_pages();
    if total_pages == 0 {
        page_label.set_text("Strana 0 / 0");
    } else {
        page_label.set_text(&format!(
            "Strana {} / {}",
            state_ref.current_page + 1,
            total_pages
        ));
    }

    previous_button.set_sensitive(state_ref.can_go_previous());
    next_button.set_sensitive(state_ref.can_go_next());
}

fn open_path(path: &Path) {
    let uri = gio::File::for_path(path).uri();
    let _ = gio::AppInfo::launch_default_for_uri(&uri, None::<&gio::AppLaunchContext>);
}

fn apply_theme(window: &ApplicationWindow, theme_id: &str) {
    for theme in THEMES {
        window.remove_css_class(theme.id);
    }

    let selected = if THEMES.iter().any(|theme| theme.id == theme_id) {
        theme_id
    } else {
        DEFAULT_THEME_ID
    };
    window.add_css_class(selected);
}

fn load_saved_theme() -> &'static str {
    let Ok(content) = fs::read_to_string(theme_settings_path()) else {
        return DEFAULT_THEME_ID;
    };
    let selected = content.trim();

    THEMES
        .iter()
        .find(|theme| theme.id == selected)
        .map(|theme| theme.id)
        .unwrap_or(DEFAULT_THEME_ID)
}

fn save_theme(theme_id: &str) -> std::io::Result<()> {
    let path = theme_settings_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(path, format!("{theme_id}\n"))
}

fn theme_settings_path() -> PathBuf {
    if let Some(appdata) = std::env::var_os("APPDATA") {
        return PathBuf::from(appdata)
            .join("rfindfiles")
            .join(CUSTOM_THEME_FILE);
    }

    if let Some(xdg_config) = std::env::var_os("XDG_CONFIG_HOME") {
        return PathBuf::from(xdg_config)
            .join("rfindfiles")
            .join(CUSTOM_THEME_FILE);
    }

    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home)
            .join(".config")
            .join("rfindfiles")
            .join(CUSTOM_THEME_FILE);
    }

    PathBuf::from(CUSTOM_THEME_FILE)
}
