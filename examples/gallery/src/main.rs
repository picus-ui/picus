use std::sync::Arc;

use bevy_math::Vec2;
use picus_core::{
    AppPicusExt, BuiltinUiAction, HasTooltip, OverlayPlacement, PicusIcon, PicusPlugin,
    ProjectionCtx, StyleClass, ToastKind, UiBadge, UiButton, UiCanvas, UiCanvasCommand,
    UiCanvasPathCommand, UiCheckbox, UiCheckboxChanged, UiColorPicker, UiColorPickerChanged,
    UiComboBox, UiComboBoxChanged, UiComboOption, UiDataColumn, UiDataRow, UiDataTable,
    UiDataTableSelectionChanged, UiDataTableSort, UiDataTableSortChanged, UiDatePicker,
    UiDatePickerChanged, UiDialog, UiEventQueue, UiFlexColumn, UiFlexRow, UiGrid, UiGridCell,
    UiGridLength, UiGroupBox, UiImage, UiLabel, UiListSelectionMode, UiListView,
    UiListViewSelectionChanged, UiMenuBar, UiMenuBarItem, UiMenuItem, UiMenuItemSelected,
    UiMultilineTextInput, UiMultilineTextInputChanged, UiPasswordInput, UiPasswordInputChanged,
    UiProgressBar, UiRadioGroup, UiRadioGroupChanged, UiRoot, UiScrollView, UiScrollViewChanged,
    UiSlider, UiSliderChanged, UiSortDirection, UiSpinner, UiSplitPane, UiSwitch, UiSwitchChanged,
    UiTabBar, UiTabChanged, UiTable, UiTextInput, UiTextInputChanged, UiThemePicker,
    UiThemePickerChanged, UiToast, UiTreeNode, UiTreeNodeToggled, UiView, apply_label_style,
    apply_widget_style,
    bevy_app::{App, Startup, Update},
    bevy_ecs::{hierarchy::ChildOf, prelude::*},
    resolve_style, resolve_style_for_classes, run_app_with_window_options, spawn_in_overlay_root,
    xilem::{
        Color,
        masonry::layout::{Dim, Length},
        style::Style as _,
        view::{FlexExt as _, flex_col, label},
        winit::{dpi::LogicalSize, error::EventLoopError},
    },
};
use shared_utils::init_logging;

const PAGE_VIEWPORT: Vec2 = Vec2::new(1040.0, 560.0);
const PAGE_CONTENT: Vec2 = Vec2::new(1040.0, 5200.0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GalleryPage {
    Buttons,
    Inputs,
    Selection,
    WindowMenu,
    MessageBox,
    Lists,
    GridView,
    Panels,
    Layout,
    Typography,
    Media,
    Shapes,
    Icons,
    Transitions,
    Overlay,
}

impl GalleryPage {
    const ALL: [Self; 15] = [
        Self::Buttons,
        Self::Inputs,
        Self::Selection,
        Self::WindowMenu,
        Self::MessageBox,
        Self::Lists,
        Self::GridView,
        Self::Panels,
        Self::Layout,
        Self::Typography,
        Self::Media,
        Self::Shapes,
        Self::Icons,
        Self::Transitions,
        Self::Overlay,
    ];

    const fn label(self) -> &'static str {
        match self {
            Self::Buttons => "Buttons",
            Self::Inputs => "Inputs",
            Self::Selection => "Selection",
            Self::WindowMenu => "Window/Menu",
            Self::MessageBox => "MessageBox",
            Self::Lists => "Lists",
            Self::GridView => "GridView",
            Self::Panels => "Panels",
            Self::Layout => "Layout",
            Self::Typography => "Typography",
            Self::Media => "Media",
            Self::Shapes => "Shapes",
            Self::Icons => "Icons",
            Self::Transitions => "Transitions",
            Self::Overlay => "Overlay",
        }
    }
}

#[derive(Resource, Debug, Clone)]
struct GalleryState {
    last_event: String,
}

impl Default for GalleryState {
    fn default() -> Self {
        Self {
            last_event: "Gallery ready. Interact with a control to see events here.".to_string(),
        }
    }
}

#[derive(Resource, Debug, Clone)]
struct GalleryRuntime {
    pages_tab_bar: Entity,
    nav_buttons: Vec<Entity>,
    open_dialog_btn: Entity,
    persistent_toast_btn: Entity,
    success_toast_btn: Entity,
    warning_toast_btn: Entity,
    error_toast_btn: Entity,
    prompt_dialog_btn: Entity,
    native_message_btn: Entity,
    popover_dialog_btn: Entity,
    burst_placeholder_btn: Entity,
}

#[derive(Component, Debug, Clone, Copy, Default)]
struct GalleryRoot;

#[derive(Component, Debug, Clone, Copy, Default)]
struct GalleryStatus;

fn class(name: &str) -> StyleClass {
    StyleClass(vec![name.to_string()])
}

fn classes(names: &[&str]) -> StyleClass {
    StyleClass(names.iter().map(|name| (*name).to_string()).collect())
}

fn project_gallery_root(_: &GalleryRoot, ctx: ProjectionCtx<'_>) -> UiView {
    let style = resolve_style(ctx.world, ctx.entity);
    let children = ctx
        .children
        .into_iter()
        .map(|child| child.into_any_flex())
        .collect::<Vec<_>>();

    Arc::new(apply_widget_style(
        flex_col(children)
            .gap(Length::px(style.layout.gap))
            .width(Dim::Stretch)
            .height(Dim::Stretch),
        &style,
    ))
}

fn project_gallery_status(_: &GalleryStatus, ctx: ProjectionCtx<'_>) -> UiView {
    let style = resolve_style(ctx.world, ctx.entity);
    let text_style = resolve_style_for_classes(ctx.world, ["gallery.note"]);
    let state = ctx.world.resource::<GalleryState>();

    Arc::new(apply_widget_style(
        apply_label_style(label(state.last_event.clone()), &text_style),
        &style,
    ))
}

fn setup_gallery(mut commands: Commands) {
    let root = commands
        .spawn((UiRoot, GalleryRoot, class("gallery.root")))
        .id();

    spawn_top_bar(&mut commands, root);

    commands.spawn((GalleryStatus, class("gallery.status"), ChildOf(root)));

    let body = commands
        .spawn((UiFlexRow, class("gallery.body"), ChildOf(root)))
        .id();

    let sidebar = commands
        .spawn((UiFlexColumn, class("gallery.sidebar"), ChildOf(body)))
        .id();

    let mut nav_buttons = Vec::new();
    for page in GalleryPage::ALL {
        let mut names = vec!["gallery.sidebar_button"];
        if page == GalleryPage::Buttons {
            names.push("gallery.sidebar_button.active");
        }
        let button = commands
            .spawn((
                UiButton::new(page.label()),
                classes(&names),
                ChildOf(sidebar),
            ))
            .id();
        nav_buttons.push(button);
    }

    let pages_tab_bar = commands
        .spawn((
            UiTabBar::new(GalleryPage::ALL.map(GalleryPage::label)).with_hidden_headers(),
            class("gallery.content_scroll"),
            ChildOf(body),
        ))
        .id();

    let open_dialog_btn = spawn_page(
        &mut commands,
        pages_tab_bar,
        GalleryPage::Buttons,
        spawn_buttons_page,
    );
    let runtime_refs = GalleryRuntime {
        pages_tab_bar,
        nav_buttons,
        open_dialog_btn,
        persistent_toast_btn: spawn_page(
            &mut commands,
            pages_tab_bar,
            GalleryPage::Inputs,
            spawn_inputs_page,
        ),
        success_toast_btn: spawn_page(
            &mut commands,
            pages_tab_bar,
            GalleryPage::Selection,
            spawn_selection_page,
        ),
        warning_toast_btn: spawn_page(
            &mut commands,
            pages_tab_bar,
            GalleryPage::WindowMenu,
            spawn_window_menu_page,
        ),
        error_toast_btn: spawn_page(
            &mut commands,
            pages_tab_bar,
            GalleryPage::MessageBox,
            spawn_message_box_page,
        ),
        prompt_dialog_btn: spawn_page(
            &mut commands,
            pages_tab_bar,
            GalleryPage::Lists,
            spawn_lists_page,
        ),
        native_message_btn: spawn_page(
            &mut commands,
            pages_tab_bar,
            GalleryPage::GridView,
            spawn_grid_view_page,
        ),
        popover_dialog_btn: spawn_page(
            &mut commands,
            pages_tab_bar,
            GalleryPage::Panels,
            spawn_panels_page,
        ),
        burst_placeholder_btn: spawn_page(
            &mut commands,
            pages_tab_bar,
            GalleryPage::Layout,
            spawn_layout_page,
        ),
    };

    spawn_page(
        &mut commands,
        pages_tab_bar,
        GalleryPage::Typography,
        spawn_typography_page,
    );
    spawn_page(
        &mut commands,
        pages_tab_bar,
        GalleryPage::Media,
        spawn_media_page,
    );
    spawn_page(
        &mut commands,
        pages_tab_bar,
        GalleryPage::Shapes,
        spawn_shapes_page,
    );
    spawn_page(
        &mut commands,
        pages_tab_bar,
        GalleryPage::Icons,
        spawn_icons_page,
    );
    spawn_page(
        &mut commands,
        pages_tab_bar,
        GalleryPage::Transitions,
        spawn_transitions_page,
    );
    spawn_page(
        &mut commands,
        pages_tab_bar,
        GalleryPage::Overlay,
        spawn_overlay_page,
    );

    commands.insert_resource(runtime_refs);
}

fn spawn_top_bar(commands: &mut Commands, root: Entity) {
    let top = commands
        .spawn((UiFlexRow, class("gallery.top_bar"), ChildOf(root)))
        .id();
    let brand = commands
        .spawn((UiFlexRow, class("gallery.brand"), ChildOf(top)))
        .id();
    commands.spawn((UiLabel::new("P"), class("gallery.logo"), ChildOf(brand)));
    let title_col = commands
        .spawn((UiFlexColumn, class("gallery.brand"), ChildOf(brand)))
        .id();
    commands.spawn((
        UiLabel::new("Picus Gallery"),
        class("gallery.title"),
        ChildOf(title_col),
    ));
    commands.spawn((
        UiLabel::new("MewUI FBA gallery inspired example, rebuilt with ECS-native Picus controls"),
        class("gallery.subtitle"),
        ChildOf(title_col),
    ));

    let tools = commands
        .spawn((UiFlexRow, class("gallery.brand"), ChildOf(top)))
        .id();
    commands.spawn((UiThemePicker::fluent(), ChildOf(tools)));
    commands.spawn((UiBadge::new("FBA parity pass"), ChildOf(tools)));
}

fn spawn_page(
    commands: &mut Commands,
    pages_tab_bar: Entity,
    page: GalleryPage,
    build: fn(&mut Commands, Entity) -> Entity,
) -> Entity {
    let scroll = commands
        .spawn((
            UiScrollView::new(PAGE_VIEWPORT, PAGE_CONTENT)
                .with_vertical_scrollbar(true)
                .with_horizontal_scrollbar(false),
            class("gallery.content_scroll"),
            ChildOf(pages_tab_bar),
        ))
        .id();
    let page_col = commands
        .spawn((UiFlexColumn, class("gallery.page"), ChildOf(scroll)))
        .id();
    commands.spawn((
        UiLabel::new(page.label()),
        class("gallery.section_title"),
        ChildOf(page_col),
    ));
    build(commands, page_col)
}

fn card(commands: &mut Commands, parent: Entity, title: &str) -> Entity {
    let card = commands
        .spawn((UiFlexColumn, class("gallery.card"), ChildOf(parent)))
        .id();
    commands.spawn((
        UiLabel::new(title),
        class("gallery.card_title"),
        ChildOf(card),
    ));
    card
}

fn grid(commands: &mut Commands, parent: Entity, columns: u32) -> Entity {
    commands
        .spawn((
            UiGrid::new(columns, 1),
            class("gallery.card_grid"),
            ChildOf(parent),
        ))
        .id()
}

fn note(commands: &mut Commands, parent: Entity, text: impl Into<String>) {
    commands.spawn((UiLabel::new(text), class("gallery.note"), ChildOf(parent)));
}

fn placeholder(commands: &mut Commands, parent: Entity, title: &str, reason: &str) {
    // Placeholder: the referenced MewUI gallery feature cannot be represented with
    // the current Picus public component set for the reason written into the card.
    let panel = commands
        .spawn((UiFlexColumn, class("gallery.placeholder"), ChildOf(parent)))
        .id();
    commands.spawn((
        UiLabel::new(title),
        class("gallery.card_title"),
        ChildOf(panel),
    ));
    commands.spawn((UiLabel::new(reason), class("gallery.note"), ChildOf(panel)));
}

fn spawn_buttons_page(commands: &mut Commands, parent: Entity) -> Entity {
    let g = grid(commands, parent, 3);

    let buttons = card(commands, g, "Buttons");
    commands.spawn((UiButton::new("Default"), ChildOf(buttons)));
    commands.spawn((
        UiButton::new("Accent"),
        class("gallery.accent_button"),
        ChildOf(buttons),
    ));
    commands.spawn((
        UiButton::new("Flat"),
        class("gallery.flat_button"),
        ChildOf(buttons),
    ));
    commands.spawn((
        UiButton::new("Danger"),
        class("gallery.danger_button"),
        ChildOf(buttons),
    ));
    let open_dialog_btn = commands
        .spawn((UiButton::new("Open Dialog"), ChildOf(buttons)))
        .id();
    note(
        commands,
        buttons,
        "Double-click and disabled button states from MewUI are placeholders below.",
    );

    let toggles = card(commands, g, "Toggle / Switch");
    commands.spawn((
        UiSwitch::new(true).with_label("Streaming"),
        ChildOf(toggles),
    ));
    commands.spawn((
        UiSwitch::new(false).with_label("Notifications"),
        ChildOf(toggles),
    ));
    commands.spawn((
        UiCheckbox::new("ToggleButton-style checkbox", true),
        ChildOf(toggles),
    ));
    commands.spawn((
        UiCheckbox::new("Unchecked toggle state", false),
        ChildOf(toggles),
    ));

    let progress = card(commands, g, "Progress");
    commands.spawn((
        UiProgressBar::determinate(0.20),
        class("gallery.progress"),
        ChildOf(progress),
    ));
    commands.spawn((
        UiProgressBar::determinate(0.65),
        class("gallery.progress"),
        ChildOf(progress),
    ));
    commands.spawn((
        UiProgressBar::indeterminate(),
        class("gallery.progress"),
        ChildOf(progress),
    ));
    commands.spawn((
        UiSlider::new(0.0, 100.0, 25.0).with_step(5.0),
        ChildOf(progress),
    ));

    placeholder(
        commands,
        g,
        "Disabled / double-click button states",
        "Picus UiButton currently exposes click events but not disabled state or double-click action routing.",
    );

    open_dialog_btn
}

fn spawn_inputs_page(commands: &mut Commands, parent: Entity) -> Entity {
    let g = grid(commands, parent, 3);

    let text = card(commands, g, "TextBox");
    commands.spawn((
        UiTextInput::new("").with_placeholder("Type your name..."),
        ChildOf(text),
    ));
    commands.spawn((UiTextInput::new("This is my name"), ChildOf(text)));
    commands.spawn((UiTextInput::new("Read/write ECS text"), ChildOf(text)));

    let password = card(commands, g, "PasswordBox");
    commands.spawn((
        UiPasswordInput::new("").with_placeholder("Password"),
        ChildOf(password),
    ));
    commands.spawn((
        UiPasswordInput::new("secret").with_mask('*'),
        ChildOf(password),
    ));
    commands.spawn((
        UiPasswordInput::new("disabled placeholder").read_only(true),
        ChildOf(password),
    ));

    let multiline = card(commands, g, "MultiLineTextBox");
    commands.spawn((
        UiMultilineTextInput::new(
            "The quick brown fox jumps over the lazy dog.\n\n- Wrap supported\n- Selection is provided by Masonry text input\n- ECS value sync is enabled",
        )
        .with_placeholder("Notes"),
        ChildOf(multiline),
    ));

    let combo = card(commands, g, "Combo / Numeric");
    let mut language = UiComboBox::new(vec![
        UiComboOption::new("rust", "Rust"),
        UiComboOption::new("csharp", "C#"),
        UiComboOption::new("swift", "Swift"),
        UiComboOption::new("kotlin", "Kotlin"),
    ])
    .with_placeholder("Pick a language");
    language.selected = 0;
    commands.spawn((language, ChildOf(combo)));
    commands.spawn((
        UiSlider::new(0.0, 100.0, 42.5).with_step(0.5),
        ChildOf(combo),
    ));
    placeholder(
        commands,
        combo,
        "NumericUpDown",
        "Picus has UiSlider but no spinner/text hybrid numeric-up-down control yet.",
    );

    let tooltip = card(commands, g, "ToolTip / Context");
    commands.spawn((
        UiButton::new("Hover for tooltip"),
        HasTooltip::new("Tooltip overlay anchored to this button."),
        ChildOf(tooltip),
    ));
    placeholder(
        commands,
        tooltip,
        "Context menu",
        "Picus has menu-bar overlays, but no right-click ContextMenu component or key gesture model yet.",
    );

    let drag_drop = card(commands, g, "Drag and Drop");
    placeholder(
        commands,
        drag_drop,
        "Window drag/drop",
        "Picus input bridge does not expose platform file-drop IDataObject/XDND events to ECS yet.",
    );

    commands
        .spawn((UiButton::new("Show persistent toast"), ChildOf(text)))
        .id()
}

fn spawn_selection_page(commands: &mut Commands, parent: Entity) -> Entity {
    let g = grid(commands, parent, 3);

    let check = card(commands, g, "CheckBox");
    commands.spawn((UiCheckbox::new("CheckBox", false), ChildOf(check)));
    commands.spawn((UiCheckbox::new("Checked", true), ChildOf(check)));
    placeholder(
        commands,
        check,
        "Three-state CheckBox",
        "UiCheckbox currently stores a bool, so indeterminate state is not represented.",
    );

    let radio = card(commands, g, "RadioButton");
    commands.spawn((
        UiRadioGroup::new(["Apple", "Banana", "Cherry", "Long long option"]).with_selected(1),
        ChildOf(radio),
    ));

    let pickers = card(commands, g, "Pickers");
    commands.spawn((UiColorPicker::new(0x60, 0xA5, 0xFA), ChildOf(pickers)));
    commands.spawn((UiDatePicker::new(2026, 5, 24), ChildOf(pickers)));
    commands.spawn((
        UiComboBox::new(vec![
            UiComboOption::new("small", "Small"),
            UiComboOption::new("medium", "Medium"),
            UiComboOption::new("large", "Large"),
        ])
        .with_placeholder("Size"),
        ChildOf(pickers),
    ));

    let list = card(commands, g, "ListBox");
    commands.spawn((
        UiListView::new((1..=8).map(|i| format!("Item {i}")))
            .with_selected(2)
            .with_item_padding(7.0),
        ChildOf(list),
    ));

    let calendar = card(commands, g, "Calendar");
    commands.spawn((UiDatePicker::new(2024, 6, 15), ChildOf(calendar)));
    placeholder(
        commands,
        calendar,
        "Always-visible calendar",
        "UiDatePicker renders its month grid only as an anchored overlay panel.",
    );

    commands
        .spawn((UiButton::new("Success Toast"), ChildOf(pickers)))
        .id()
}

fn spawn_window_menu_page(commands: &mut Commands, parent: Entity) -> Entity {
    let g = grid(commands, parent, 2);

    let menu = card(commands, g, "MenuBar");
    let menu_bar = commands.spawn((UiMenuBar, ChildOf(menu))).id();
    commands.spawn((
        UiMenuBarItem::new(
            "File",
            [
                UiMenuItem::new("New File", "file.new"),
                UiMenuItem::new("Open...", "file.open"),
                UiMenuItem::new("Save", "file.save"),
                UiMenuItem::new("Exit", "file.exit"),
            ],
        ),
        ChildOf(menu_bar),
    ));
    commands.spawn((
        UiMenuBarItem::new(
            "Edit",
            [
                UiMenuItem::new("Copy", "edit.copy"),
                UiMenuItem::new("Paste", "edit.paste"),
                UiMenuItem::new("Select All", "edit.select_all"),
            ],
        ),
        ChildOf(menu_bar),
    ));
    commands.spawn((
        UiMenuBarItem::new(
            "View",
            [
                UiMenuItem::new("Zoom In", "view.zoom_in"),
                UiMenuItem::new("Zoom Out", "view.zoom_out"),
                UiMenuItem::new("Reset Zoom", "view.reset_zoom"),
            ],
        ),
        ChildOf(menu_bar),
    ));
    note(
        commands,
        menu,
        "Submenus and shortcuts from MewUI are represented as flat menu items.",
    );

    let dialogs = card(commands, g, "Window APIs");
    placeholder(
        commands,
        dialogs,
        "Transparent/custom/native windows",
        "Picus run helpers expose a Bevy-owned primary window, but no per-example secondary window or custom chrome API.",
    );
    placeholder(
        commands,
        dialogs,
        "File dialogs",
        "The framework re-exports rfd, but no ECS-native UiFileDialog component exists for this gallery page.",
    );
    let warning = commands
        .spawn((UiButton::new("Warning Toast"), ChildOf(dialogs)))
        .id();

    let access = card(commands, g, "Access Keys");
    placeholder(
        commands,
        access,
        "Access keys and accelerators",
        "Picus currently routes pointer/text input but does not expose menu access-key registration.",
    );

    warning
}

fn spawn_message_box_page(commands: &mut Commands, parent: Entity) -> Entity {
    let g = grid(commands, parent, 3);

    let modal = card(commands, g, "Dialog");
    let error_btn = commands
        .spawn((UiButton::new("Error Toast"), ChildOf(modal)))
        .id();
    commands.spawn((UiButton::new("Open modal dialog"), ChildOf(modal)));
    note(
        commands,
        modal,
        "Picus UiDialog covers the modal overlay portion of MewUI MessageBox.",
    );

    let prompts = card(commands, g, "Prompt Dialog");
    placeholder(
        commands,
        prompts,
        "Prompt text dialog",
        "UiDialog has title/body/dismiss fields but no built-in input slot or confirm/cancel result contract.",
    );

    let native = card(commands, g, "Native Message Hook");
    placeholder(
        commands,
        native,
        "Native message hook",
        "Masonry runtime is abstracted behind Picus; platform-native message hooks are not public API.",
    );

    error_btn
}

fn spawn_lists_page(commands: &mut Commands, parent: Entity) -> Entity {
    let g = grid(commands, parent, 2);

    let list = card(commands, g, "ListView");
    commands.spawn((
        UiListView::new((1..=20).map(|i| format!("Gallery item {i}")))
            .with_selected(4)
            .with_item_height(30.0)
            .with_item_padding(6.0),
        ChildOf(list),
    ));

    let multi = card(commands, g, "Multi-selection List");
    commands.spawn((
        UiListView::new(["Alpha", "Beta", "Gamma", "Delta", "Epsilon", "Zeta"])
            .with_selected_indices([1, 3])
            .with_item_padding(7.0),
        ChildOf(multi),
    ));

    let tree = card(commands, g, "TreeView");
    let root_node = commands
        .spawn((UiTreeNode::new("Root").expanded(), ChildOf(tree)))
        .id();
    let docs = commands
        .spawn((UiTreeNode::new("Documents").expanded(), ChildOf(root_node)))
        .id();
    commands.spawn((UiTreeNode::new("report.pdf"), ChildOf(docs)));
    commands.spawn((UiTreeNode::new("notes.txt"), ChildOf(docs)));
    let src = commands
        .spawn((UiTreeNode::new("src").expanded(), ChildOf(root_node)))
        .id();
    commands.spawn((UiTreeNode::new("main.rs"), ChildOf(src)));
    commands.spawn((UiTreeNode::new("widgets.rs"), ChildOf(src)));

    let table = card(commands, g, "Table");
    commands.spawn((
        UiTable::new(["Name", "Role", "Status", "Score"])
            .with_row(["Alice Chen", "Engineer", "Active", "98"])
            .with_row(["Bob Smith", "Designer", "Away", "85"])
            .with_row(["Carol Davis", "Manager", "Busy", "91"]),
        class("gallery.table"),
        ChildOf(table),
    ));

    commands
        .spawn((UiButton::new("Prompt Placeholder"), ChildOf(list)))
        .id()
}

fn spawn_grid_view_page(commands: &mut Commands, parent: Entity) -> Entity {
    let g = grid(commands, parent, 2);

    let data = card(commands, g, "DataTable / GridView");
    commands.spawn((
        UiDataTable::new([
            UiDataColumn::new("file", "File").width(180.0),
            UiDataColumn::new("kind", "Kind"),
            UiDataColumn::new("size", "Size"),
            UiDataColumn::new("changed", "Changed"),
        ])
        .with_row(UiDataRow::new(
            "1",
            ["fba_gallery.cs", "C# sample", "120 KB", "2026-05-21"],
        ))
        .with_row(UiDataRow::new(
            "2",
            ["main.rs", "Rust example", "42 KB", "2026-05-24"],
        ))
        .with_row(UiDataRow::new(
            "3",
            ["gallery.ron", "Theme", "12 KB", "2026-05-24"],
        ))
        .with_row(UiDataRow::new(
            "4",
            ["fluent_theme.ron", "Theme bundle", "58 KB", "2026-05-23"],
        ))
        .with_sort(UiDataTableSort::new(0, UiSortDirection::Ascending))
        .with_selection_mode(UiListSelectionMode::Single)
        .striped(true)
        .with_row_height(32.0),
        ChildOf(data),
    ));

    let visual = card(commands, g, "Template Columns");
    note(
        commands,
        visual,
        "String-backed selectable rows, sortable headers, widths, selected row, and stripes are supported.",
    );
    placeholder(
        commands,
        visual,
        "Cell templates / images",
        "UiDataTable currently stores text cells, so per-cell templates and embedded images are not public yet.",
    );

    commands
        .spawn((UiButton::new("Native Message Placeholder"), ChildOf(visual)))
        .id()
}

fn spawn_panels_page(commands: &mut Commands, parent: Entity) -> Entity {
    let g = grid(commands, parent, 2);

    let group_box = card(commands, g, "GroupBox / Cards");
    let inner = commands
        .spawn((UiGroupBox::new("Nested group"), ChildOf(group_box)))
        .id();
    commands.spawn((
        UiLabel::new("Labels and controls can be grouped."),
        ChildOf(inner),
    ));
    commands.spawn((UiCheckbox::new("Inside a group", true), ChildOf(inner)));

    let split = card(commands, g, "SplitPane");
    let pane = commands
        .spawn((UiSplitPane::new(0.42), ChildOf(split)))
        .id();
    let left = commands
        .spawn((UiFlexColumn, class("gallery.split_panel"), ChildOf(pane)))
        .id();
    commands.spawn((UiLabel::new("Left panel"), ChildOf(left)));
    commands.spawn((
        UiListView::new(["One", "Two", "Three"]).with_selected(0),
        ChildOf(left),
    ));
    let right = commands
        .spawn((UiFlexColumn, class("gallery.split_panel"), ChildOf(pane)))
        .id();
    commands.spawn((UiLabel::new("Right panel"), ChildOf(right)));
    commands.spawn((UiTextInput::new("Resizable split content"), ChildOf(right)));

    let tabs = card(commands, g, "Tabs");
    let tab_bar = commands
        .spawn((
            UiTabBar::new(["Details", "Settings", "Logs"]),
            ChildOf(tabs),
        ))
        .id();
    commands.spawn((UiLabel::new("Details tab content"), ChildOf(tab_bar)));
    commands.spawn((UiCheckbox::new("Enable option", true), ChildOf(tab_bar)));
    commands.spawn((
        UiMultilineTextInput::new("Log line 1\nLog line 2"),
        ChildOf(tab_bar),
    ));

    let popover = card(commands, g, "Popover");
    let pop_btn = commands
        .spawn((UiButton::new("Open popover dialog"), ChildOf(popover)))
        .id();
    note(
        commands,
        popover,
        "Picus popovers are used by combo boxes, menus, pickers, and tooltips.",
    );

    pop_btn
}

fn spawn_layout_page(commands: &mut Commands, parent: Entity) -> Entity {
    let g = grid(commands, parent, 2);

    let flex = card(commands, g, "StackPanel / Flex");
    let row = commands.spawn((UiFlexRow, ChildOf(flex))).id();
    commands.spawn((UiBadge::new("Auto"), ChildOf(row)));
    commands.spawn((UiBadge::new("Stretch"), ChildOf(row)));
    commands.spawn((UiBadge::new("Gap"), ChildOf(row)));
    commands.spawn((UiTextInput::new("Horizontal row"), ChildOf(flex)));

    let grid_card = card(commands, g, "Grid");
    let layout_grid = commands
        .spawn((
            UiGrid::new(3, 3)
                .with_column_tracks([
                    UiGridLength::auto(),
                    UiGridLength::star(1.0),
                    UiGridLength::px(160.0),
                ])
                .with_row_tracks([
                    UiGridLength::px(40.0),
                    UiGridLength::star(1.0),
                    UiGridLength::auto(),
                ])
                .show_grid_lines(true),
            ChildOf(grid_card),
        ))
        .id();
    commands.spawn((
        UiLabel::new("Span 2 columns"),
        class("gallery.swatch.blue"),
        UiGridCell::new(0, 0).with_span(2, 1),
        ChildOf(layout_grid),
    ));
    commands.spawn((
        UiLabel::new("Auto"),
        class("gallery.swatch.green"),
        UiGridCell::new(2, 0),
        ChildOf(layout_grid),
    ));
    commands.spawn((
        UiLabel::new("Star"),
        class("gallery.swatch.gold"),
        UiGridCell::new(0, 1).with_span(3, 1),
        ChildOf(layout_grid),
    ));

    let canvas_panel = card(commands, g, "Canvas / Absolute");
    commands.spawn((
        sample_canvas(),
        class("gallery.canvas"),
        ChildOf(canvas_panel),
    ));
    placeholder(
        commands,
        canvas_panel,
        "Right/bottom attached canvas children",
        "UiCanvasPosition stores right/bottom intent, but the current projector only applies left/top offsets.",
    );

    commands
        .spawn((UiButton::new("Confetti Placeholder"), ChildOf(canvas_panel)))
        .id()
}

fn spawn_typography_page(commands: &mut Commands, parent: Entity) -> Entity {
    let g = grid(commands, parent, 2);

    let scale = card(commands, g, "Text Scale");
    commands.spawn((
        UiLabel::new("Display / Hero"),
        class("gallery.typo.hero"),
        ChildOf(scale),
    ));
    commands.spawn((
        UiLabel::new("Title text"),
        class("gallery.typo.title"),
        ChildOf(scale),
    ));
    commands.spawn((
        UiLabel::new("Body text for gallery descriptions and form copy."),
        class("gallery.typo.body"),
        ChildOf(scale),
    ));
    commands.spawn((
        UiLabel::new("Caption / secondary metadata"),
        class("gallery.typo.caption"),
        ChildOf(scale),
    ));

    let cjk = card(commands, g, "CJK / Unicode");
    commands.spawn((
        UiLabel::new("Picus Gallery: 骨 / 骨 / こんにちは / 你好"),
        class("gallery.typo.title"),
        ChildOf(cjk),
    ));
    note(
        commands,
        cjk,
        "Fonts are bridged through Picus; this gallery registers the bundled Noto Sans font.",
    );

    let wrapping = card(commands, g, "Text wrapping");
    commands.spawn((
        UiMultilineTextInput::new(
            "MewUI TypographyPage covers families, weight, wrapping, alignment, and editable text. Picus exposes most text through labels and text inputs today.",
        )
        .read_only(true),
        ChildOf(wrapping),
    ));

    placeholder(
        commands,
        g,
        "Rich text runs",
        "UiLabel is plain text; mixed inline spans/weights/colors require a richer text component.",
    );

    parent
}

fn spawn_media_page(commands: &mut Commands, parent: Entity) -> Entity {
    let g = grid(commands, parent, 2);

    let generated = card(commands, g, "Image");
    commands.spawn((
        generated_image(),
        class("gallery.image"),
        ChildOf(generated),
    ));
    note(
        commands,
        generated,
        "The source image is generated in-memory so the example is self-contained.",
    );

    let empty = card(commands, g, "Image fallback");
    commands.spawn((
        UiImage::empty().with_alt_text("Image resource unavailable"),
        class("gallery.image"),
        ChildOf(empty),
    ));
    placeholder(
        commands,
        empty,
        "Remote image loading",
        "MewUI downloads sample resources at runtime; this example avoids cargo run/network dependency for gallery startup.",
    );

    let canvas = card(commands, g, "Canvas media");
    commands.spawn((sample_canvas(), class("gallery.canvas"), ChildOf(canvas)));

    placeholder(
        commands,
        g,
        "Video / animated image",
        "Picus has bitmap image and canvas components, but no video or animated image component yet.",
    );

    parent
}

fn spawn_shapes_page(commands: &mut Commands, parent: Entity) -> Entity {
    let g = grid(commands, parent, 2);

    let primitives = card(commands, g, "Shapes");
    commands.spawn((
        sample_canvas(),
        class("gallery.canvas"),
        ChildOf(primitives),
    ));

    let fills = card(commands, g, "Brushes");
    commands.spawn((
        UiLabel::new("Red"),
        class("gallery.swatch.red"),
        ChildOf(fills),
    ));
    commands.spawn((
        UiLabel::new("Green"),
        class("gallery.swatch.green"),
        ChildOf(fills),
    ));
    commands.spawn((
        UiLabel::new("Blue"),
        class("gallery.swatch.blue"),
        ChildOf(fills),
    ));
    commands.spawn((
        UiLabel::new("Gold"),
        class("gallery.swatch.gold"),
        ChildOf(fills),
    ));

    placeholder(
        commands,
        g,
        "Gradient / transform brushes",
        "UiCanvasCommand supports solid-color fills and strokes; gradient brush stops are not exposed.",
    );

    placeholder(
        commands,
        g,
        "Shape hit testing",
        "Canvas drawing is visual only; per-shape pointer hit testing is not a public component contract.",
    );

    parent
}

fn spawn_icons_page(commands: &mut Commands, parent: Entity) -> Entity {
    let g = grid(commands, parent, 4);

    for (name, icon) in [
        ("Check", PicusIcon::Check),
        ("Chevron Down", PicusIcon::ChevronDown),
        ("Chevron Up", PicusIcon::ChevronUp),
        ("Chevron Right", PicusIcon::ChevronRight),
        ("Circle", PicusIcon::Circle),
        ("Circle Dot", PicusIcon::CircleDot),
        ("Close", PicusIcon::X),
        ("Theme", PicusIcon::SunMoon),
    ] {
        let c = card(commands, g, name);
        commands.spawn((
            UiLabel::new(icon.glyph().to_string()),
            class("gallery.icon"),
            ChildOf(c),
        ));
        commands.spawn((UiLabel::new(name), class("gallery.icon_label"), ChildOf(c)));
    }

    placeholder(
        commands,
        parent,
        "Full Icons.xaml browser",
        "Picus exposes a curated PicusIcon enum backed by Lucide font bytes; it does not parse MewUI Icons.xaml path resources.",
    );

    parent
}

fn spawn_transitions_page(commands: &mut Commands, parent: Entity) -> Entity {
    let g = grid(commands, parent, 2);

    let theme = card(commands, g, "Theme transitions");
    commands.spawn((UiThemePicker::fluent(), ChildOf(theme)));
    note(
        commands,
        theme,
        "Changing theme variants exercises style target sync and color transition animation.",
    );
    commands.spawn((
        UiButton::new("Hover / press transition"),
        class("gallery.accent_button"),
        ChildOf(theme),
    ));
    commands.spawn((
        UiSwitch::new(true).with_label("Animated switch target"),
        ChildOf(theme),
    ));

    let loading = card(commands, g, "Motion");
    commands.spawn((UiSpinner::new(), ChildOf(loading)));
    commands.spawn((
        UiSpinner::new().with_label("Loading resources"),
        ChildOf(loading),
    ));
    commands.spawn((
        UiProgressBar::indeterminate(),
        class("gallery.progress"),
        ChildOf(loading),
    ));

    placeholder(
        commands,
        g,
        "ConfettiOverlay",
        "MewUI draws timer-driven custom particles; Picus has static UiCanvas commands but no retained animated canvas component API yet.",
    );

    placeholder(
        commands,
        g,
        "Storyboard transitions",
        "Current public styling exposes color/scale transitions, not arbitrary property storyboards.",
    );

    parent
}

fn spawn_overlay_page(commands: &mut Commands, parent: Entity) -> Entity {
    let g = grid(commands, parent, 3);

    let dialogs = card(commands, g, "Dialog");
    commands.spawn((UiButton::new("Open Dialog"), ChildOf(dialogs)));
    note(
        commands,
        dialogs,
        "Modal dialog overlays are available through UiDialog.",
    );

    let toast = card(commands, g, "Toasts");
    commands.spawn((UiButton::new("Info Toast"), ChildOf(toast)));
    commands.spawn((UiButton::new("Success Toast"), ChildOf(toast)));
    commands.spawn((UiButton::new("Warning Toast"), ChildOf(toast)));
    commands.spawn((UiButton::new("Error Toast"), ChildOf(toast)));

    let anchored = card(commands, g, "Anchored overlays");
    commands.spawn((
        UiComboBox::new(vec![
            UiComboOption::new("top", "Top"),
            UiComboOption::new("bottom", "Bottom"),
            UiComboOption::new("start", "Start"),
        ])
        .with_placeholder("Combo overlay"),
        ChildOf(anchored),
    ));
    commands.spawn((UiColorPicker::new(0xE5, 0x48, 0x4D), ChildOf(anchored)));
    commands.spawn((UiDatePicker::new(2026, 5, 24), ChildOf(anchored)));
    commands.spawn((
        UiButton::new("Tooltip source"),
        HasTooltip::new("Tooltip overlay follows its source entity."),
        ChildOf(anchored),
    ));

    placeholder(
        commands,
        g,
        "Manual overlay positioning",
        "OverlayPlacement supports anchored and viewport placements; arbitrary manual pixel placement is not a public component.",
    );

    parent
}

fn sample_canvas() -> UiCanvas {
    UiCanvas::new()
        .with_alt_text("Canvas shape sample")
        .with_command(UiCanvasCommand::FillCanvas {
            color: Color::from_rgb8(0x1E, 0x29, 0x3B),
        })
        .with_command(UiCanvasCommand::FillRoundedRect {
            x: 16.0,
            y: 16.0,
            width: 180.0,
            height: 90.0,
            radius: 12.0,
            color: Color::from_rgb8(0x25, 0x63, 0xEB),
        })
        .with_command(UiCanvasCommand::StrokeRoundedRect {
            x: 16.0,
            y: 16.0,
            width: 180.0,
            height: 90.0,
            radius: 12.0,
            color: Color::from_rgb8(0xBF, 0xDB, 0xFE),
            stroke_width: 2.0,
        })
        .with_command(UiCanvasCommand::FillCircle {
            cx: 238.0,
            cy: 62.0,
            radius: 42.0,
            color: Color::from_rgb8(0xF9, 0x73, 0x16),
        })
        .with_command(UiCanvasCommand::Line {
            x1: 24.0,
            y1: 132.0,
            x2: 296.0,
            y2: 132.0,
            color: Color::from_rgb8(0xF8, 0xFA, 0xFC),
            stroke_width: 3.0,
        })
        .with_command(UiCanvasCommand::FillPath {
            commands: vec![
                UiCanvasPathCommand::MoveTo { x: 42.0, y: 168.0 },
                UiCanvasPathCommand::LineTo { x: 118.0, y: 142.0 },
                UiCanvasPathCommand::LineTo { x: 164.0, y: 190.0 },
                UiCanvasPathCommand::ClosePath,
            ],
            color: Color::from_rgb8(0x22, 0xC5, 0x5E),
        })
        .with_command(UiCanvasCommand::StrokePath {
            commands: vec![
                UiCanvasPathCommand::MoveTo { x: 190.0, y: 170.0 },
                UiCanvasPathCommand::CubicTo {
                    x1: 214.0,
                    y1: 132.0,
                    x2: 266.0,
                    y2: 208.0,
                    x: 296.0,
                    y: 156.0,
                },
            ],
            color: Color::from_rgb8(0xE0, 0xE7, 0xFF),
            stroke_width: 4.0,
        })
}

fn generated_image() -> UiImage {
    let width = 320_u32;
    let height = 180_u32;
    let mut rgba = Vec::with_capacity(width as usize * height as usize * 4);
    for y in 0..height {
        for x in 0..width {
            let fx = x as f32 / (width - 1) as f32;
            let fy = y as f32 / (height - 1) as f32;
            let r = (42.0 + fx * 160.0) as u8;
            let g = (90.0 + fy * 120.0) as u8;
            let b = (180.0 - fx * 70.0 + fy * 40.0).clamp(0.0, 255.0) as u8;
            rgba.extend_from_slice(&[r, g, b, 255]);
        }
    }
    UiImage::from_rgba8(width, height, rgba).with_alt_text("Generated Picus media sample")
}

fn drain_gallery_events(world: &mut World) {
    let Some(rt) = world.get_resource::<GalleryRuntime>().cloned() else {
        return;
    };

    let builtin_events = world
        .resource_mut::<UiEventQueue>()
        .drain_actions::<BuiltinUiAction>();
    for event in builtin_events {
        if !matches!(event.action, BuiltinUiAction::Clicked) {
            continue;
        }

        if let Some(index) = rt
            .nav_buttons
            .iter()
            .position(|button| *button == event.entity)
        {
            set_gallery_page(world, &rt, index);
            update_status(
                world,
                format!(
                    "Navigation: switched to {}",
                    GalleryPage::ALL[index].label()
                ),
            );
        } else if event.entity == rt.open_dialog_btn {
            spawn_dialog(
                world,
                "Button Dialog",
                "This replaces MewUI's basic MessageBox with Picus UiDialog.",
            );
        } else if event.entity == rt.persistent_toast_btn {
            spawn_toast(
                world,
                "Persistent info toast. Close it manually.",
                ToastKind::Info,
                0.0,
            );
        } else if event.entity == rt.success_toast_btn {
            spawn_toast(
                world,
                "Selection page success toast.",
                ToastKind::Success,
                2.4,
            );
        } else if event.entity == rt.warning_toast_btn {
            spawn_toast(
                world,
                "Window/Menu placeholder warning.",
                ToastKind::Warning,
                3.2,
            );
        } else if event.entity == rt.error_toast_btn {
            spawn_toast(world, "MessageBox error toast.", ToastKind::Error, 3.2);
        } else if event.entity == rt.prompt_dialog_btn {
            spawn_dialog(
                world,
                "Prompt Placeholder",
                "Picus UiDialog does not yet expose an input slot, so the prompt sample is represented here.",
            );
        } else if event.entity == rt.native_message_btn {
            spawn_dialog(
                world,
                "Native Hook Placeholder",
                "Platform-native message hooks are not part of the public Picus runtime API.",
            );
        } else if event.entity == rt.popover_dialog_btn {
            spawn_dialog(
                world,
                "Popover Note",
                "Anchored overlays are implemented by combo boxes, menus, color pickers, date pickers, and tooltips.",
            );
        } else if event.entity == rt.burst_placeholder_btn {
            spawn_toast(
                world,
                "Confetti placeholder: animated retained canvas is not public yet.",
                ToastKind::Warning,
                3.5,
            );
        }
    }

    for event in world
        .resource_mut::<UiEventQueue>()
        .drain_actions::<UiThemePickerChanged>()
    {
        update_status(
            world,
            format!(
                "Theme picker {:?}: selected {} ({})",
                event.action.picker, event.action.selected, event.action.variant
            ),
        );
    }

    for event in world
        .resource_mut::<UiEventQueue>()
        .drain_actions::<UiCheckboxChanged>()
    {
        update_status(
            world,
            format!(
                "CheckBox {:?}: {}",
                event.action.checkbox,
                if event.action.checked {
                    "checked"
                } else {
                    "unchecked"
                }
            ),
        );
    }

    for event in world
        .resource_mut::<UiEventQueue>()
        .drain_actions::<UiSwitchChanged>()
    {
        update_status(
            world,
            format!(
                "Switch {:?}: {}",
                event.action.switch,
                if event.action.on { "on" } else { "off" }
            ),
        );
    }

    for event in world
        .resource_mut::<UiEventQueue>()
        .drain_actions::<UiSliderChanged>()
    {
        update_status(
            world,
            format!(
                "Slider {:?}: value {:.2}",
                event.action.slider, event.action.value
            ),
        );
    }

    for event in world
        .resource_mut::<UiEventQueue>()
        .drain_actions::<UiTextInputChanged>()
    {
        update_status(
            world,
            format!("TextInput {:?}: {}", event.action.input, event.action.value),
        );
    }

    for event in world
        .resource_mut::<UiEventQueue>()
        .drain_actions::<UiPasswordInputChanged>()
    {
        update_status(
            world,
            format!(
                "PasswordInput {:?}: {} chars",
                event.action.input,
                event.action.value.chars().count()
            ),
        );
    }

    for event in world
        .resource_mut::<UiEventQueue>()
        .drain_actions::<UiMultilineTextInputChanged>()
    {
        update_status(
            world,
            format!(
                "MultilineTextInput {:?}: {} chars",
                event.action.input,
                event.action.value.chars().count()
            ),
        );
    }

    for event in world
        .resource_mut::<UiEventQueue>()
        .drain_actions::<UiRadioGroupChanged>()
    {
        update_status(
            world,
            format!(
                "RadioGroup {:?}: index {}",
                event.action.group, event.action.selected
            ),
        );
    }

    for event in world
        .resource_mut::<UiEventQueue>()
        .drain_actions::<UiComboBoxChanged>()
    {
        update_status(
            world,
            format!(
                "ComboBox {:?}: {} ({})",
                event.action.combo, event.action.selected, event.action.value
            ),
        );
    }

    for event in world
        .resource_mut::<UiEventQueue>()
        .drain_actions::<UiColorPickerChanged>()
    {
        update_status(
            world,
            format!(
                "ColorPicker {:?}: #{:02X}{:02X}{:02X}",
                event.action.picker, event.action.r, event.action.g, event.action.b
            ),
        );
    }

    for event in world
        .resource_mut::<UiEventQueue>()
        .drain_actions::<UiDatePickerChanged>()
    {
        update_status(
            world,
            format!(
                "DatePicker {:?}: {:04}-{:02}-{:02}",
                event.action.picker, event.action.year, event.action.month, event.action.day
            ),
        );
    }

    for event in world
        .resource_mut::<UiEventQueue>()
        .drain_actions::<UiListViewSelectionChanged>()
    {
        update_status(
            world,
            format!(
                "ListView {:?}: selected {:?} rows {:?}",
                event.action.list_view, event.action.selected, event.action.selected_indices
            ),
        );
    }

    for event in world
        .resource_mut::<UiEventQueue>()
        .drain_actions::<UiDataTableSelectionChanged>()
    {
        update_status(
            world,
            format!(
                "DataTable {:?}: selected {:?}",
                event.action.table, event.action.selected_rows
            ),
        );
    }

    for event in world
        .resource_mut::<UiEventQueue>()
        .drain_actions::<UiDataTableSortChanged>()
    {
        update_status(
            world,
            format!(
                "DataTable {:?}: sorted column {} {:?}",
                event.action.table, event.action.sort.column, event.action.sort.direction
            ),
        );
    }

    for event in world
        .resource_mut::<UiEventQueue>()
        .drain_actions::<UiTreeNodeToggled>()
    {
        update_status(
            world,
            format!(
                "TreeNode {:?}: {}",
                event.action.node,
                if event.action.is_expanded {
                    "expanded"
                } else {
                    "collapsed"
                }
            ),
        );
    }

    for event in world
        .resource_mut::<UiEventQueue>()
        .drain_actions::<UiMenuItemSelected>()
    {
        update_status(
            world,
            format!(
                "Menu item {:?}: {}",
                event.action.bar_item, event.action.value
            ),
        );
    }

    for event in world
        .resource_mut::<UiEventQueue>()
        .drain_actions::<UiTabChanged>()
    {
        if event.action.bar != rt.pages_tab_bar {
            update_status(
                world,
                format!(
                    "TabBar {:?}: index {}",
                    event.action.bar, event.action.active
                ),
            );
        }
    }

    for event in world
        .resource_mut::<UiEventQueue>()
        .drain_actions::<UiScrollViewChanged>()
    {
        update_status(
            world,
            format!(
                "ScrollView {:?}: offset ({:.1}, {:.1})",
                event.action.scroll_view,
                event.action.scroll_offset.x,
                event.action.scroll_offset.y
            ),
        );
    }
}

fn set_gallery_page(world: &mut World, rt: &GalleryRuntime, page: usize) {
    if let Some(mut tab_bar) = world.get_mut::<UiTabBar>(rt.pages_tab_bar) {
        tab_bar.active = page.min(tab_bar.tabs.len().saturating_sub(1));
    }

    for (index, button) in rt.nav_buttons.iter().copied().enumerate() {
        let class_list = if index == page {
            classes(&["gallery.sidebar_button", "gallery.sidebar_button.active"])
        } else {
            class("gallery.sidebar_button")
        };
        if world.get_entity(button).is_ok() {
            world.entity_mut(button).insert(class_list);
        }
    }
}

fn spawn_dialog(world: &mut World, title: &str, body: &str) {
    spawn_in_overlay_root(world, (UiDialog::new(title, body).with_fixed_width(460.0),));
    update_status(world, format!("Dialog opened: {title}"));
}

fn spawn_toast(world: &mut World, message: &str, kind: ToastKind, duration: f32) {
    spawn_in_overlay_root(
        world,
        (UiToast::new(message)
            .with_kind(kind)
            .with_duration(duration)
            .with_min_width(320.0)
            .with_max_width(480.0)
            .with_placement(OverlayPlacement::BottomEnd),),
    );
    update_status(world, format!("Toast: {message}"));
}

fn update_status(world: &mut World, text: String) {
    if let Some(mut state) = world.get_resource_mut::<GalleryState>() {
        state.last_event = text;
    }
}

picus_core::impl_ui_component_template!(GalleryRoot, project_gallery_root);
picus_core::impl_ui_component_template!(GalleryStatus, project_gallery_status);

fn build_gallery_app() -> App {
    init_logging();

    let mut app = App::new();
    app.add_plugins(PicusPlugin)
        .load_style_sheet_ron(include_str!("../assets/themes/gallery.ron"))
        .register_xilem_font(picus_core::SyncAssetSource::Bytes(include_bytes!(
            "../../../assets/fonts/NotoSans-Regular.ttf",
        )))
        .insert_resource(GalleryState::default())
        .register_ui_component::<GalleryRoot>()
        .register_ui_component::<GalleryStatus>()
        .add_systems(Startup, setup_gallery)
        .add_systems(
            Update,
            drain_gallery_events
                .after(picus_core::handle_widget_actions)
                .after(picus_core::handle_overlay_actions),
        );

    app
}

fn main() -> Result<(), EventLoopError> {
    run_app_with_window_options(build_gallery_app(), "Picus Gallery", |options| {
        options.with_initial_inner_size(LogicalSize::new(1360.0, 760.0))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_gallery_theme_ron_parses() {
        picus_core::parse_stylesheet_ron(include_str!("../assets/themes/gallery.ron"))
            .expect("embedded gallery stylesheet should parse");
    }

    #[test]
    fn gallery_pages_match_mewui_gallery_sections() {
        let labels = GalleryPage::ALL.map(GalleryPage::label);
        assert_eq!(
            labels,
            [
                "Buttons",
                "Inputs",
                "Selection",
                "Window/Menu",
                "MessageBox",
                "Lists",
                "GridView",
                "Panels",
                "Layout",
                "Typography",
                "Media",
                "Shapes",
                "Icons",
                "Transitions",
                "Overlay",
            ],
        );
    }
}
