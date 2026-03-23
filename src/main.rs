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
use rfindfiles::{Config, find_files, parse_date, parse_extensions, parse_folders};

const RESULTS_PER_PAGE: usize = 50;
const CUSTOM_THEME_FILE: &str = "theme.txt";
const CUSTOM_LANGUAGE_FILE: &str = "language.txt";
const COMMON_EXTENSIONS: [&str; 6] = ["pdf", "docx", "xlsx", "txt", "jpg", "png"];
const THEMES: [ThemeDefinition; 4] = [
    ThemeDefinition {
        id: "theme-sand",
        label_en: "Sand",
        label_cs: "Pisek",
    },
    ThemeDefinition {
        id: "theme-forest",
        label_en: "Forest",
        label_cs: "Les",
    },
    ThemeDefinition {
        id: "theme-night",
        label_en: "Night",
        label_cs: "Noc",
    },
    ThemeDefinition {
        id: "theme-ink",
        label_en: "Ink",
        label_cs: "Inkoust",
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum Language {
    En,
    Cs,
}

impl Language {
    fn code(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::Cs => "cs",
        }
    }

    fn from_code(value: &str) -> Self {
        match value.trim() {
            "cs" => Self::Cs,
            _ => Self::En,
        }
    }

    fn title(self) -> &'static str {
        match self {
            Self::En => "Rust File Finder",
            Self::Cs => "Rust File Finder",
        }
    }

    fn theme_menu(self) -> &'static str {
        match self {
            Self::En => "Theme",
            Self::Cs => "Vzhled",
        }
    }

    fn language_menu(self) -> &'static str {
        match self {
            Self::En => "Language",
            Self::Cs => "Jazyk",
        }
    }

    fn folders(self) -> &'static str {
        match self {
            Self::En => "Folders",
            Self::Cs => "Slozky",
        }
    }

    fn folders_placeholder(self) -> &'static str {
        match self {
            Self::En => "Folders separated by commas",
            Self::Cs => "Slozky oddelene carkou",
        }
    }

    fn add_folder(self) -> &'static str {
        match self {
            Self::En => "Add Folder",
            Self::Cs => "Pridat slozku",
        }
    }

    fn file_types(self) -> &'static str {
        match self {
            Self::En => "File Types",
            Self::Cs => "Pripony",
        }
    }

    fn custom(self) -> &'static str {
        match self {
            Self::En => "Custom:",
            Self::Cs => "Vlastni:",
        }
    }

    fn custom_placeholder(self) -> &'static str {
        match self {
            Self::En => "Custom extensions, e.g. md,csv,json",
            Self::Cs => "Vlastni pripony, napr. md,csv,json",
        }
    }

    fn date(self) -> &'static str {
        match self {
            Self::En => "Date",
            Self::Cs => "Datum",
        }
    }

    fn from(self) -> &'static str {
        match self {
            Self::En => "From",
            Self::Cs => "Od",
        }
    }

    fn to(self) -> &'static str {
        match self {
            Self::En => "To",
            Self::Cs => "Do",
        }
    }

    fn pick_date(self) -> &'static str {
        match self {
            Self::En => "Pick date",
            Self::Cs => "Vybrat datum",
        }
    }

    fn clear(self) -> &'static str {
        match self {
            Self::En => "Clear",
            Self::Cs => "Vymazat",
        }
    }

    fn search(self) -> &'static str {
        match self {
            Self::En => "Search",
            Self::Cs => "Hledat",
        }
    }

    fn previous(self) -> &'static str {
        match self {
            Self::En => "Previous",
            Self::Cs => "Predchozi",
        }
    }

    fn next(self) -> &'static str {
        match self {
            Self::En => "Next",
            Self::Cs => "Dalsi",
        }
    }

    fn file_dialog_title(self) -> &'static str {
        match self {
            Self::En => "Select folder",
            Self::Cs => "Vyberte slozku",
        }
    }

    fn select(self) -> &'static str {
        match self {
            Self::En => "Select",
            Self::Cs => "Vybrat",
        }
    }

    fn cancel(self) -> &'static str {
        match self {
            Self::En => "Cancel",
            Self::Cs => "Zrusit",
        }
    }

    fn found_files(self, count: usize) -> String {
        match self {
            Self::En => format!("Files found: {count}"),
            Self::Cs => format!("Nalezeno souboru: {count}"),
        }
    }

    fn page(self, current: usize, total: usize) -> String {
        match self {
            Self::En => format!("Page {current} / {total}"),
            Self::Cs => format!("Strana {current} / {total}"),
        }
    }

    fn empty_page(self) -> &'static str {
        match self {
            Self::En => "Page 0 / 0",
            Self::Cs => "Strana 0 / 0",
        }
    }

    fn validation(self, error: ValidationError) -> &'static str {
        match (self, error) {
            (Self::En, ValidationError::MissingFolder) => "Select at least one existing folder.",
            (Self::Cs, ValidationError::MissingFolder) => {
                "Vyberte alespon jednu existujici slozku."
            }
            (Self::En, ValidationError::MissingExtension) => "Select at least one file type.",
            (Self::Cs, ValidationError::MissingExtension) => "Vyberte alespon jeden typ souboru.",
            (Self::En, ValidationError::InvalidDate) => {
                "Invalid date. Use the picker to select a valid date."
            }
            (Self::Cs, ValidationError::InvalidDate) => {
                "Neplatne datum. Pouzijte vyber platneho data."
            }
            (Self::En, ValidationError::InvalidDateRange) => {
                "'From' must be earlier than or equal to 'To'."
            }
            (Self::Cs, ValidationError::InvalidDateRange) => {
                "Pole 'Od' musi byt mensi nebo rovno poli 'Do'."
            }
        }
    }
}

#[derive(Clone, Copy)]
struct ThemeDefinition {
    id: &'static str,
    label_en: &'static str,
    label_cs: &'static str,
}

impl ThemeDefinition {
    fn label(self, language: Language) -> &'static str {
        match language {
            Language::En => self.label_en,
            Language::Cs => self.label_cs,
        }
    }
}

#[derive(Clone, Copy)]
enum ValidationError {
    MissingFolder,
    MissingExtension,
    InvalidDate,
    InvalidDateRange,
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

struct UiTextRefs {
    title_label: Label,
    theme_menu_button: MenuButton,
    language_menu_button: MenuButton,
    folders_label: Label,
    folders_entry: Entry,
    add_folder_button: Button,
    file_types_label: Label,
    custom_label: Label,
    custom_entry: Entry,
    date_label: Label,
    date_from_label: Label,
    date_from_button: MenuButton,
    date_from_clear: Button,
    date_to_label: Label,
    date_to_button: MenuButton,
    date_to_clear: Button,
    search_button: Button,
    previous_button: Button,
    next_button: Button,
    page_label: Label,
    theme_option_buttons: Vec<(ThemeDefinition, Button)>,
}

fn main() {
    let app = Application::builder()
        .application_id("com.example.rfindfiles")
        .build();

    app.connect_activate(build_ui);
    app.run();
}

fn build_ui(app: &Application) {
    let language = Rc::new(RefCell::new(load_saved_language()));

    let window = ApplicationWindow::builder()
        .application(app)
        .title(language.borrow().title())
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
    let title_label = Label::new(Some(language.borrow().title()));
    headerbar.set_title_widget(Some(&title_label));
    let theme_menu_button = MenuButton::new();
    let theme_menu_popover = Popover::new();
    let theme_menu_box = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(6)
        .margin_top(8)
        .margin_bottom(8)
        .margin_start(8)
        .margin_end(8)
        .build();
    let theme_option_buttons = build_theme_menu_buttons(
        &window,
        &theme_menu_popover,
        &theme_menu_box,
        *language.borrow(),
    );
    theme_menu_popover.set_child(Some(&theme_menu_box));
    theme_menu_button.set_popover(Some(&theme_menu_popover));
    headerbar.pack_end(&theme_menu_button);

    let language_menu_button = MenuButton::new();
    let language_menu_popover = Popover::new();
    let language_menu_box = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(6)
        .margin_top(8)
        .margin_bottom(8)
        .margin_start(8)
        .margin_end(8)
        .build();
    let en_button = Button::with_label("en");
    let cs_button = Button::with_label("cs");
    language_menu_box.append(&en_button);
    language_menu_box.append(&cs_button);
    language_menu_popover.set_child(Some(&language_menu_box));
    language_menu_button.set_popover(Some(&language_menu_popover));
    headerbar.pack_end(&language_menu_button);

    window.set_titlebar(Some(&headerbar));
    apply_theme(&window, load_saved_theme());

    let folders_entry = Entry::builder().hexpand(true).build();
    let add_folder_button = Button::new();
    let search_button = Button::new();
    search_button.add_css_class("accent-button");
    let previous_button = Button::new();
    let next_button = Button::new();
    let page_label = Label::new(None);
    let status_label = Label::new(None);
    status_label.set_xalign(0.0);

    let extensions_box = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(12)
        .build();
    let extension_checkboxes = build_extension_checkboxes(&extensions_box);
    let custom_label = Label::new(None);
    let custom_extensions_entry = Entry::builder().hexpand(true).build();
    extensions_box.append(&custom_label);
    extensions_box.append(&custom_extensions_entry);

    let (date_from_widget, date_from_label, date_from_button, date_from_clear) =
        build_date_picker();
    let (date_to_widget, date_to_label, date_to_button, date_to_clear) = build_date_picker();

    let folders_label = Label::new(None);
    let folders_row = build_row(
        &folders_label,
        &[
            folders_entry.clone().upcast::<gtk::Widget>(),
            add_folder_button.clone().upcast::<gtk::Widget>(),
        ],
    );

    let file_types_label = Label::new(None);
    let filters_row = build_row(
        &file_types_label,
        &[extensions_box.clone().upcast::<gtk::Widget>()],
    );

    let date_label = Label::new(None);
    let dates_row = build_row(
        &date_label,
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
    let ui_refs = Rc::new(UiTextRefs {
        title_label,
        theme_menu_button,
        language_menu_button,
        folders_label,
        folders_entry: folders_entry.clone(),
        add_folder_button: add_folder_button.clone(),
        file_types_label,
        custom_label,
        custom_entry: custom_extensions_entry.clone(),
        date_label,
        date_from_label,
        date_from_button: date_from_button.clone(),
        date_from_clear: date_from_clear.clone(),
        date_to_label,
        date_to_button: date_to_button.clone(),
        date_to_clear: date_to_clear.clone(),
        search_button: search_button.clone(),
        previous_button: previous_button.clone(),
        next_button: next_button.clone(),
        page_label: page_label.clone(),
        theme_option_buttons,
    });
    apply_language(
        &ui_refs,
        *language.borrow(),
        &state.borrow(),
        None,
        None,
        &window,
    );

    {
        let window = window.clone();
        let folders_entry = folders_entry.clone();
        let status_label = status_label.clone();
        let language = language.clone();
        add_folder_button.connect_clicked(move |_| {
            let current_language = *language.borrow();
            let dialog = FileChooserNative::new(
                Some(current_language.file_dialog_title()),
                Some(&window),
                FileChooserAction::SelectFolder,
                Some(current_language.select()),
                Some(current_language.cancel()),
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
        let language = language.clone();
        search_button.connect_clicked(move |_| {
            let current_language = *language.borrow();
            let extensions_raw =
                collect_extensions(&extension_checkboxes, &custom_extensions_entry);
            match build_config(
                &folders_entry.text(),
                &extensions_raw,
                &button_date_value(&date_from_button, current_language),
                &button_date_value(&date_to_button, current_language),
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
                        current_language,
                    );
                    status_label.set_text(&current_language.found_files(total));
                }
                Err(err) => {
                    state.borrow_mut().set_results(Vec::new());
                    refresh_results(
                        &results_list,
                        &page_label,
                        &previous_button,
                        &next_button,
                        &state,
                        current_language,
                    );
                    status_label.set_text(current_language.validation(err));
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
        let language = language.clone();
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
                *language.borrow(),
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
        let language = language.clone();
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
                *language.borrow(),
            );
        });
    }

    {
        let ui_refs = ui_refs.clone();
        let language = language.clone();
        let state = state.clone();
        let status_label = status_label.clone();
        let window = window.clone();
        let language_menu_popover = language_menu_popover.clone();
        en_button.connect_clicked(move |_| {
            *language.borrow_mut() = Language::En;
            let _ = save_language(Language::En);
            apply_language(
                &ui_refs,
                Language::En,
                &state.borrow(),
                Some(&status_label),
                status_label.text().as_str().into(),
                &window,
            );
            language_menu_popover.popdown();
        });
    }

    {
        let ui_refs = ui_refs.clone();
        let language = language.clone();
        let state = state.clone();
        let status_label = status_label.clone();
        let window = window.clone();
        let language_menu_popover = language_menu_popover.clone();
        cs_button.connect_clicked(move |_| {
            *language.borrow_mut() = Language::Cs;
            let _ = save_language(Language::Cs);
            apply_language(
                &ui_refs,
                Language::Cs,
                &state.borrow(),
                Some(&status_label),
                status_label.text().as_str().into(),
                &window,
            );
            language_menu_popover.popdown();
        });
    }

    refresh_results(
        &results_list,
        &page_label,
        &previous_button,
        &next_button,
        &state,
        *language.borrow(),
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

fn build_theme_menu_buttons(
    window: &ApplicationWindow,
    popover: &Popover,
    container: &GtkBox,
    language: Language,
) -> Vec<(ThemeDefinition, Button)> {
    let mut buttons = Vec::with_capacity(THEMES.len());

    for theme in THEMES {
        let button = Button::with_label(theme.label(language));
        let window = window.clone();
        let popover = popover.clone();
        button.connect_clicked(move |_| {
            apply_theme(&window, theme.id);
            let _ = save_theme(theme.id);
            popover.popdown();
        });
        container.append(&button);
        buttons.push((theme, button));
    }

    buttons
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

fn build_date_picker() -> (GtkBox, Label, MenuButton, Button) {
    let wrapper = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(6)
        .build();
    let label = Label::new(None);
    let button = MenuButton::new();
    let popover = Popover::new();
    let calendar = Calendar::new();
    let clear_button = Button::new();

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

    popover.set_child(Some(&calendar));
    button.set_popover(Some(&popover));
    wrapper.append(&label);
    wrapper.append(&button);
    wrapper.append(&clear_button);

    (wrapper, label, button, clear_button)
}

fn build_row(label: &Label, widgets: &[gtk::Widget]) -> GtkBox {
    let row = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(8)
        .build();

    label.set_width_chars(10);
    label.set_xalign(0.0);
    label.set_halign(Align::Start);
    row.append(label);

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

fn button_date_value(button: &MenuButton, language: Language) -> String {
    match button.label() {
        Some(label) if label != language.pick_date() => label.to_string(),
        _ => String::new(),
    }
}

fn build_config(
    folders_raw: &str,
    extensions_raw: &str,
    date_from_raw: &str,
    date_to_raw: &str,
) -> Result<Config, ValidationError> {
    let folders = parse_folders(folders_raw);
    if folders.is_empty() {
        return Err(ValidationError::MissingFolder);
    }

    let extensions = parse_extensions(extensions_raw);
    if extensions.is_empty() {
        return Err(ValidationError::MissingExtension);
    }

    let date_from = if date_from_raw.trim().is_empty() {
        None
    } else {
        Some(parse_date(date_from_raw, false).map_err(|_| ValidationError::InvalidDate)?)
    };
    let date_to = if date_to_raw.trim().is_empty() {
        None
    } else {
        Some(parse_date(date_to_raw, true).map_err(|_| ValidationError::InvalidDate)?)
    };

    if let (Some(from), Some(to)) = (date_from, date_to) {
        if from > to {
            return Err(ValidationError::InvalidDateRange);
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
    language: Language,
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
        page_label.set_text(language.empty_page());
    } else {
        page_label.set_text(&language.page(state_ref.current_page + 1, total_pages));
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

fn apply_language(
    ui: &UiTextRefs,
    language: Language,
    pagination: &PaginationState,
    status_label: Option<&Label>,
    existing_status: Option<&str>,
    window: &ApplicationWindow,
) {
    window.set_title(Some(language.title()));
    ui.title_label.set_text(language.title());
    ui.theme_menu_button.set_label(language.theme_menu());
    ui.language_menu_button.set_label(language.language_menu());
    ui.folders_label.set_text(language.folders());
    ui.folders_entry
        .set_placeholder_text(Some(language.folders_placeholder()));
    ui.add_folder_button.set_label(language.add_folder());
    ui.file_types_label.set_text(language.file_types());
    ui.custom_label.set_text(language.custom());
    ui.custom_entry
        .set_placeholder_text(Some(language.custom_placeholder()));
    ui.date_label.set_text(language.date());
    ui.date_from_label.set_text(language.from());
    ui.date_to_label.set_text(language.to());
    ui.date_from_clear.set_label(language.clear());
    ui.date_to_clear.set_label(language.clear());
    ui.search_button.set_label(language.search());
    ui.previous_button.set_label(language.previous());
    ui.next_button.set_label(language.next());

    if button_label_is_date_placeholder(&ui.date_from_button) {
        ui.date_from_button.set_label(language.pick_date());
    }
    if button_label_is_date_placeholder(&ui.date_to_button) {
        ui.date_to_button.set_label(language.pick_date());
    }

    for (theme, button) in &ui.theme_option_buttons {
        button.set_label(theme.label(language));
    }

    if pagination.total_pages() == 0 {
        ui.page_label.set_text(language.empty_page());
    } else {
        ui.page_label
            .set_text(&language.page(pagination.current_page + 1, pagination.total_pages()));
    }

    if let (Some(status_label), Some(existing_status)) = (status_label, existing_status) {
        if existing_status.is_empty() {
            return;
        }

        if existing_status.starts_with("Files found: ")
            || existing_status.starts_with("Nalezeno souboru: ")
        {
            status_label.set_text(&language.found_files(pagination.all_results.len()));
        }
    }
}

fn button_label_is_date_placeholder(button: &MenuButton) -> bool {
    matches!(button.label(), Some(label) if label == Language::En.pick_date() || label == Language::Cs.pick_date())
}

fn load_saved_theme() -> &'static str {
    let Ok(content) = fs::read_to_string(settings_path(CUSTOM_THEME_FILE)) else {
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
    let path = settings_path(CUSTOM_THEME_FILE);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(path, format!("{theme_id}\n"))
}

fn load_saved_language() -> Language {
    let Ok(content) = fs::read_to_string(settings_path(CUSTOM_LANGUAGE_FILE)) else {
        return Language::En;
    };

    Language::from_code(&content)
}

fn save_language(language: Language) -> std::io::Result<()> {
    let path = settings_path(CUSTOM_LANGUAGE_FILE);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(path, format!("{}\n", language.code()))
}

fn settings_path(file_name: &str) -> PathBuf {
    if let Some(appdata) = std::env::var_os("APPDATA") {
        return PathBuf::from(appdata).join("rfindfiles").join(file_name);
    }

    if let Some(xdg_config) = std::env::var_os("XDG_CONFIG_HOME") {
        return PathBuf::from(xdg_config).join("rfindfiles").join(file_name);
    }

    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home)
            .join(".config")
            .join("rfindfiles")
            .join(file_name);
    }

    PathBuf::from(file_name)
}
