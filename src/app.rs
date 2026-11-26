use iced::{
    widget::{button, column, container, row, scrollable, text, text_editor, text_input, Space},
    Alignment, Element, Length, Task, Theme,
};
use std::collections::{BTreeSet, HashMap, HashSet};

use crate::history::{History, Snapshot};
use std::path::{Path, PathBuf};

use crate::{
    config::{Config, ShortcutConfig, ShortcutTarget},
    editor::EditorState,
    preferences::{PreferencesMessage, PreferencesState},
    preview::{self, PreviewKind, ViewMode},
    search::{SearchMessage, SearchState},
    sidebar::{SidebarAction, SidebarMessage, SidebarState},
    tabs::{fingerprint, CachedBuffer, DiskStamp, Loaded, TextFormat},
    text_ops::{self, comment_style},
    theme as se_theme,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TopMenu {
    File,
    Edit,
    View,
    Help,
}

/// The places a name is used, with the highlighted one (arrows and Enter).
#[derive(Debug, Clone, PartialEq)]
pub struct RefList {
    pub word: String,
    pub items: Vec<crate::lsp::Reference>,
    pub selected: usize,
    pub state: RefState,
}

/// What the usages popup shows: it opens as soon as the search starts.
#[derive(Debug, Clone, PartialEq)]
pub enum RefState {
    Searching,
    /// The search ended with nothing to list: why.
    Empty(String),
    Ready,
}

impl RefList {
    pub(crate) fn searching(word: String) -> Self {
        Self {
            word,
            items: Vec::new(),
            selected: 0,
            state: RefState::Searching,
        }
    }
}

/// The language servers that can be run, and the ones that can be removed.
pub type Servers = (
    std::collections::HashSet<&'static str>,
    std::collections::HashSet<&'static str>,
);

/// What the Git shortcuts ask for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitShortcut {
    /// The Commit tab, to write the message.
    Commit,
    Push,
    Pull,
}

/// What the arrows of a diff tab do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffNav {
    PrevChange,
    NextChange,
    PrevFile,
    NextFile,
    ToggleSide,
    /// The mouse on the rows of the diff.
    Pick(crate::diff_view::DiffPointer),
    /// The diff was scrolled: (offset, height shown).
    Scrolled(i32, i32),
    CopyPicked,
    PickAll,
    /// Put the old version of a block of changes in the file.
    Apply(usize),
    /// Stage (or unstage) a block, or the lines picked; blanks left out of
    /// the diff; more or less context.
    StageBlock(usize),
    StageSelection,
    ToggleBlanks,
    CycleContext,
    /// A commit's diff against the current file, and back.
    ToggleCompare,
    /// Opens the file of the diff in the editor.
    OpenFile,
}

/// A conflicted file open in an editor tab (`merge://N/path`).
pub type MergeTabs = HashMap<PathBuf, crate::diff_view::merge::MergeTab>;

/// A diff shown in an editor tab (`diff://N/title`): the files to walk
/// through, where it is.
pub struct DiffTabState {
    pub viewer: crate::diff_view::Viewer,
    /// A tab of a plugin: the plugin, its id for the tab, and the file on
    /// the disk of each diff. Its buttons go to the plugin.
    pub owner: Option<(String, String, Vec<Option<PathBuf>>)>,
}

/// The column of annotations of a file (the blame of the Git plugin).
pub struct BlameState {
    pub path: PathBuf,
    pub annotations: crate::code_editor::Annotations,
}

/// What the current file is compared with to mark its changed lines.
pub struct MarksState {
    pub path: PathBuf,
    /// What a plugin said it is compared with (`None`: new, all added).
    pub base: Option<String>,
    pub marks: crate::diff::marks::ChangeMarks,
    /// The marks as the editor takes them, shared between frames.
    pub paint: [std::sync::Arc<Vec<usize>>; 3],
    /// The buffer (see [`ArdoiseApp::buffer_stamp`]) the marks were computed for.
    pub rev: (u64, u64),
}

/// Which modal overlay (if any) is currently shown.
/// What can be changed in how a blame is made.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlameOption {
    Blanks,
    MovesInFile,
    MovesAcross,
    /// The commit of the line is left out.
    HideLine,
    ShowHidden,
}

/// What the status bar's choice windows are about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextOption {
    Encoding,
    LineEnding,
    Indent,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ActiveOverlay {
    None,
    /// The encoding, line endings or indentation of the file.
    TextOptions(TextOption),
    /// Two texts side by side (their names).
    Compare(String, String),
    /// A misspelled word, and what could replace it.
    Spelling(Box<spelling::Suggestion>),
    /// The languages of the dictionaries found.
    SpellLanguages,
    /// The tables of a schema and the keys between them.
    /// The new name of the symbol under the cursor, being typed.
    Rename(String),
    /// Search everywhere (Shift twice).
    Everywhere,
    About,
    /// The first launch: the basic settings and the plugins to install.
    Welcome,
    /// Every release.
    Changelog,
    /// The releases since the given version, shown after an update.
    WhatsNew(String),
    GotoLine,
    LanguagePicker,
    /// Where a name is used: the list to jump from.
    References(RefList),
    UpdateChecking,
    UpdateUpToDate,
    /// The new version, and what it changes.
    UpdateAvailable(String, Vec<crate::changelog::Entry>),
    UpdateDownloading,
    UpdateInstalling,
    UpdateError(String),
    /// Closing this tab would discard unsaved changes.
    ConfirmClose(PathBuf),
    /// Closing these tabs would discard unsaved changes in some of them.
    ConfirmCloseMany(Vec<PathBuf>, usize),
    /// The session couldn't be written: quitting would lose unsaved buffers.
    ConfirmQuit(String),
    /// A folder has programs of its own (`node_modules/.bin`…): may they run?
    ConfirmTrust(PathBuf),
    /// Saving would overwrite changes made to the file outside the editor.
    ConfirmOverwrite(PathBuf),
    /// A scratch file is about to be deleted for good.
    ConfirmScratchDelete(PathBuf),
    /// A text looked for in the files of the projects.
    FindInFiles,
    /// A name asked for a file or folder of a project.
    ProjectName(project_ops::NameDialog),
    /// A folder to delete for good, everything in it.
    ConfirmFolderDelete(PathBuf),
    /// A file or folder dragged onto a folder, to move there.
    ConfirmMove(PathBuf, PathBuf),
    /// A file to delete for good.
    /// The file, and the boxes the plugins added: (plugin, box as ticked).
    ConfirmFileDelete(
        PathBuf,
        Vec<(String, ardoise_plugin::protocol::DeleteOption)>,
    ),
    /// A question of a plugin.
    PluginConfirm(Box<plugins::PluginQuestion>),
    /// A plugin to uninstall, and whether its data goes too.
    ConfirmPluginUninstall(String, bool),
}

#[derive(Debug, Clone)]
pub enum Message {
    EditorAction(text_editor::Action),
    /// The plugins, and the Plugins page of the preferences.
    Plugin(crate::plugins::PluginMsg),
    Sidebar(SidebarMessage),
    Search(SearchMessage),
    Preferences(PreferencesMessage),
    OpenFile,
    /// A file chosen by the user (dialog, command line, recent files).
    FileOpened(Result<Loaded, String>),
    /// A file read to switch to its tab; superseded by a later switch.
    TabRead(Result<Loaded, String>),
    /// The open file changed on disk and was read again.
    FileReloaded(Result<Loaded, String>),
    /// Throws the changes of the tab away and reads the file again.
    RevertFile,
    SaveFile,
    SaveAll,
    SaveFileAs,
    /// The path picked in "Save as", with the tab it was started from.
    SaveAsChosen(Option<PathBuf>, Result<PathBuf, String>),
    /// The tab the save was started from, and the path + exact text written.
    FileSaved(Option<PathBuf>, Result<(PathBuf, String), String>),
    NewFile,
    CloseFile,
    /// Closes every tab but this one (none: all of them).
    CloseTabs(Option<PathBuf>),
    /// Closes every tab but the active one.
    CloseOtherTabs,
    /// The user confirmed closing these tabs although some have unsaved changes.
    ConfirmDiscardMany(Vec<PathBuf>),
    ToggleSearch,
    ToggleSidebar,
    /// A tab was dragged from a place to another.
    TabMoved(usize, usize),
    /// A group of tabs folded or opened; the color of a project.
    /// The arrows and the picks of the diff of a comparison.
    CompareDiff(DiffNav),
    TabGroupToggle(Option<PathBuf>),
    TabGroupClose(Option<PathBuf>),
    ProjectColor(PathBuf, usize),
    /// A click on a file of the tree: it opens in the preview tab (a second
    /// click, at once, keeps it open).
    TreeOpenFile(PathBuf),
    /// The preview tab becomes a tab like the others.
    KeepTab,
    /// Two clicks on a tab (the preview one stays then).
    TabDoubleClick(usize),
    /// The middle button closed the tab at this place.
    CloseTabAt(usize),
    /// Opens again the tab closed last.
    ReopenTab,
    /// Asks for a file of the projects by its name.
    OpenGotoFile,
    /// Looks for a text in the files of the projects.
    OpenFindInFiles,
    /// The files of the projects, listed in the background.
    FinderFiles(Vec<(PathBuf, String)>),
    FindQuery(String),
    FindToggleCase,
    FindToggleRegex,
    /// Replace in files, and the rest of the window: the replacement, whole
    /// words, keeping the case, find or replace, the context, the file mask,
    /// where, subfolders, a list under the fields, a past search, the pin, a
    /// match picked, replacing one or all (and the confirmation), undoing,
    /// the matches to the Results panel.
    OpenReplaceInFiles,
    ReplaceInFolder(PathBuf),
    FindReplaceText(String),
    FindToggleWords,
    FindToggleMultiline,
    FindIgnored(bool),
    FindLines(text_editor::Action),
    FindTogglePreserve,
    FindMode(bool),
    FindContext(crate::find_replace::Context),
    FindMaskOn(bool),
    FindMask(String),
    FindScope(finder::Scope),
    FindPickDir,
    ToggleHighContrast,
    OpenRemote,
    SpellFix,
    SpellChecked(Option<spelling::SpellChecked>),
    /// The dictionary of a language, read in the background.
    /// The suggestions of the speller for the word at the cursor.
    SpellSuggested(spelling::Suggestion),
    /// The speller learnt a word (or why not).
    SpellLearned(Option<String>),
    /// The dictionaries of the speller.
    SpellLanguagesLoaded(Vec<String>),
    /// The dictionaries, for the Preferences (no picker).
    SpellChoices(Vec<String>),
    SpellReplace(String),
    SpellAdd,
    SpellIgnore,
    OpenSpellLanguages,
    SpellLanguage(String),
    CompareClipboard(Option<String>),
    CompareDisk,
    ComparePickFile,
    CompareFilePicked(Result<(PathBuf, String), String>),
    /// The file on disk, read to be compared with the editor.
    CompareDiskRead(Result<String, String>),
    /// A file read again (as another encoding, or to drop the changes).
    Reloaded(PathBuf, bool, Result<Loaded, String>),
    AskCompareClipboard,
    OpenTodos,
    TodosFound(Result<Vec<finder::Occurrence>, String>),
    OpenTaskPicker,
    IpcPoll,
    TargetOpened(crate::ipc::Target, Result<Loaded, String>),
    NewWindow,
    /// The folder may run its own programs.
    TrustFolder(PathBuf),
    RenameSymbol,
    /// The names the file declares, in a list to pick from.
    OpenOutline,
    OutlineLoaded(PathBuf, Result<Vec<crate::lsp::SymbolHit>, String>),
    RenameInput(String),
    RenameConfirm,
    Renamed(u64, Result<Vec<crate::lsp::FileEdits>, String>),
    SymbolHover(Option<(usize, usize, iced::Point)>),
    SymbolHoverLoaded(u64, iced::Point, Result<Option<String>, String>),
    LargeMove(LargeMove),
    LargeLoaded(PathBuf, Result<(String, u64, u64), String>),
    /// The part of a large file in front, read again because the file changed.
    LargeRefreshed(PathBuf, Result<(String, u64, u64), String>),
    LineOp(text_ops::LineOp),
    CaseOp(text_ops::CaseOp),
    ConvertIndent(bool),
    OpenTextOptions(TextOption),
    SetEncoding(crate::tabs::Encoding),
    ReloadEncoding(crate::tabs::Encoding),
    SetLineEnding(crate::tabs::LineEnding),
    SetIndent(bool, usize),
    DetectIndent,
    AddNextOccurrence(bool),
    AddCursor(bool),
    Zoom(i32),
    FindDirText(String),
    FindDirChosen(PathBuf),
    FindPreviewAction(text_editor::Action),
    FindPreviewMenu(finder::PreviewMenu),
    FindPreviewPasted(Option<String>),
    /// A replacement in many files is done (what it did, the files on disk).
    /// The open folders of the tree, read in the background.
    /// The text of the file of the preview of a search, read in the background.
    FinderPreviewRead(PathBuf, String),
    ProjectsReloaded(Vec<crate::projects::LoadedDir>),
    ReplaceDone(Box<finder::Replaced>, Vec<finder::DiskDone>),
    /// A rename is done in the files on disk too.
    RenameDone(Box<RenameOutcome>, Vec<DiskEdited>),
    FindDirPicked(Option<PathBuf>),
    FindRecursive,
    FindMenu(Option<finder::Menu>),
    FindHistory(String),
    FindPin,
    FindSelect(usize),
    FindScrolled(f32),
    /// Leave the match of this line out of "Replace all" (or put it back).
    FindExcludeAt(usize),
    FindReplaceOne,
    FindReplaceAll,
    FindReplaceConfirm(bool),
    FindUndoReplace,
    FindToWindow,
    Results(results::ResultsMsg),
    /// What a search found (its generation, the places or what went wrong).
    FindResults(u64, Result<Vec<finder::Occurrence>, String>),
    /// The pause after the last key is over: the search for `generation` may start.
    FindDue(u64),
    FindOpen(Option<usize>),
    /// Looks in this folder only (from the tree's menu).
    FindInFolder(PathBuf),
    /// The list of every open tab, opened from its button (where it was pressed).
    OpenTabList(iced::Point),
    CloseTabList,
    /// A tab picked in that list.
    PickTab(PathBuf),
    /// Opens or closes a panel of the sidebar from the bar on its left.
    TogglePanel(crate::sidebar::Panel),
    /// A setting of the lint, changed in the Preferences.
    Lint(crate::lint::Setting),
    /// A run of the linters is done (its generation, the text it read).
    LintDone(u64, lint::Key, lint::Outcome),
    /// The pointer rests on a problem (and where its card goes), or left it.
    LintHover(Option<(usize, iced::Point)>),
    /// Completion: the shortcut, what a server told about it, its answer,
    /// where the list hangs in the window, a click on an item, the wheel
    /// over the list, an item's documentation, the import an accepted item
    /// needs, whether the server still loads the project.
    CompletionInvoke,
    CompletionInfo(
        (String, PathBuf),
        Result<crate::lsp::CompletionInfo, String>,
    ),
    CompletionAnswer(
        u64,
        Box<(
            crate::completion::Base,
            Result<crate::lsp::Completions, String>,
        )>,
    ),
    CompletionAnchor(crate::code_editor::Spot, crate::code_editor::Spot),
    /// The help on the parameters of a call: asked with the shortcut, its
    /// answer, the next or previous signature of an overloaded function.
    HintInvoke,
    HintAnswer(u64, Result<Option<serde_json::Value>, String>),
    CompletionPick(usize),
    CompletionWheel(f32),
    CompletionResolved(u64, usize, Result<serde_json::Value, String>),
    CompletionImport(u64, Result<serde_json::Value, String>),
    CompletionBusy(u64, bool),
    LintCardHover(bool),
    LintCloseCheck(u64),
    /// The fixes a language server has for a problem.
    LintFixesLoaded(lint::Key, usize, Result<Vec<crate::lint::Fix>, String>),
    /// Applies a fix (of a problem, by number) to the text.
    LintApply(usize, usize),
    LintFixAll,
    /// Fixes the problems of one source (a linter, a server) only.
    LintFixSource(String),
    LintFixedAll(lint::Key, String),
    LintFixFile(PathBuf),
    /// Looks at the file now.
    LintNow,
    /// Opens the Preferences on the Lint page.
    OpenLintSettings,
    /// Asks for a folder to add to the sidebar as a project.
    AddProject,
    ProjectPicked(Option<PathBuf>),
    /// Opens or closes a folder of the tree.
    ToggleProjectFolder(PathBuf),
    /// Takes a project out of the sidebar (nothing is deleted).
    RemoveProject(PathBuf),
    RefreshProjects,
    /// Opens the folders of the tree down to a file.
    ShowInProjects(PathBuf),
    /// Closes every folder of the trees.
    CollapseProjects,
    /// A line of the tree pressed (it is picked, and may be dragged), the
    /// pointer over another line, the button let go.
    TreePress(PathBuf),
    TreeHover(PathBuf),
    TreeRelease,
    /// Moves a file or folder into a folder (after the question).
    MoveEntry(PathBuf, PathBuf),
    /// The tree was scrolled: how far, and how high it is.
    TreeScrolled(f32, f32),
    /// The names left out of the trees (as typed: separated by commas).
    SetProjectsExclude(String),
    /// A terminal that starts in this folder.
    OpenTerminalAt(PathBuf),
    /// Shows a file or folder in the system's file manager.
    RevealInFileManager(PathBuf),
    /// Asks a name for a copy of a file or folder.
    DuplicateEntry(PathBuf),
    /// The copy of a file or folder is made (the copy, how it went).
    EntryDuplicated(PathBuf, Result<(), String>),
    /// Asks a name for a file (false) or a folder (true) in a folder.
    NewProjectEntry(PathBuf, bool),
    RenameProjectEntry(PathBuf),
    ProjectNameChanged(String),
    ProjectNameSubmit,
    AskDeleteFolder(PathBuf),
    DeleteFolder(PathBuf),
    FolderDeleted(Result<PathBuf, String>),
    /// Puts a path on the clipboard.
    CopyPath(PathBuf),
    /// What Git ignores is left out of the trees (or shown again).
    ToggleHideIgnored,
    /// The cursor goes to the next problem (false: the previous one).
    LintNext(bool),
    /// Splits the editor in two (side by side, or one above the other).
    Split(bool),
    /// Shows a file in a pane of the split (the editor is split first).
    SplitWith(PathBuf, bool),
    /// Closes the active pane.
    CloseSplit,
    ClosePane(split::PaneId),
    /// A pane takes the focus.
    SplitFocus(split::PaneId),
    /// Something done in a pane that has not the focus.
    SplitAction(split::PaneId, text_editor::Action),
    /// The edge of the file list is grabbed, dragged (x of the pointer) and let go.
    /// The branches (a shortcut of the Git plugin).
    OpenBranchSwitcher,
    /// A Git shortcut: the Git plugin does it.
    GitShortcut(GitShortcut),
    /// The API panel and its requests.
    /// The Preferences window is taken hold of (moved, resized), followed
    /// to where the pointer goes, and let go.
    PrefsDragStart(crate::preferences::PrefsGrab),
    PrefsDragMove(iced::Point),
    PrefsDragEnd,
    SidebarResizeStart,
    SidebarResize(f32),
    SidebarResizeEnd,
    PanelResizeStart,
    PanelResize(f32),
    PanelResizeEnd,
    /// Delete a file for good: ask first, then do it.
    AskDeleteFile(PathBuf),
    DeleteFile(PathBuf),
    FileDeleted(Result<(), String>),
    TogglePreferences,
    ThemeChanged(bool),
    /// Switches between the dark and the light theme.
    ToggleTheme,
    InsertTab,
    ToggleMenu(TopMenu),
    /// A click outside the open menu: it closes (nothing else happens).
    CloseMenu,
    // Git: the Git plugin's panel opens or closes.
    ToggleGit,
    /// The buttons of a diff tab.
    DiffTab(DiffNav),
    // Conflicts
    Merge(crate::diff_view::merge::MergeMsg),
    MergeApplied(PathBuf, Result<(), String>),
    // Blame: the Git plugin's.
    BlameFile,
    CloseBlame,
    BlameOption(BlameOption),
    BlameLineClicked(usize),
    /// A run mark of a plugin clicked in the gutter (line from 0).
    RunMarkClicked(usize),
    /// Shows the file as it was before the commit of the line that was
    /// right-clicked in the blame column, with its own blame.
    BlamePrevious,
    /// The bar of a changed line was clicked: the button that puts the change
    /// back opens under it (or closes).
    ChangeBar(usize),
    RevertChange,
    CloseRevert,
    // Terminal
    ToggleTerminal,
    // Scratch files
    ToggleScratch,
    // Bookmarks panel
    ToggleBookmarkPanel,
    BookmarkClick(bookmark_panel::Target),
    /// The lines of bookmarked files, read in the background.
    BookmarkTexts(Vec<(PathBuf, Vec<String>)>),
    BookmarkFold(PathBuf),
    BookmarkDelete(bookmark_panel::Target),
    /// Bookmarks a file as a whole, or takes it out.
    ToggleFileBookmark(PathBuf),
    ScratchName(String),
    ScratchExt(crate::scratch::Ext),
    ScratchCreate,
    ScratchOpen(PathBuf),
    ScratchAskDelete(PathBuf),
    ScratchDelete(PathBuf),
    PickScratchDir,
    ScratchDirChosen(Option<PathBuf>),
    /// Keeps the scratch files in this folder ("" for the default one).
    SetScratchDir(String),
    /// The pointer entered a menu title: switches menus if one is already open.
    HoverMenu(TopMenu),
    /// Resolves the on-screen position of an open top menu's button, so its
    /// dropdown can be anchored under it regardless of label/font/DPI.
    MenuBounds(TopMenu, Option<iced::Rectangle>),
    SelectAll,
    Quit,
    About,
    ContextCopy,
    ContextCut,
    ContextPaste,
    ContextDelete,
    ClipboardContent(Option<String>),
    FormatFile,
    FormatSelection,
    FileFormatted(FormatJob, Result<String, String>),
    SelectionFormatted(FormatJob, Result<String, String>),
    CloseErrorPanel,
    Undo,
    Redo,
    AutoSave,
    /// The watcher heard a change on the disk.
    FsChanged,
    // Raw key press forwarded from the subscription (non-capturing fn pointer)
    KeyPressed {
        key: String,
        ctrl: bool,
        shift: bool,
        alt: bool,
    },
    EscapePressed,
    /// Enter pressed while a search field might own it; `shift` goes back.
    SearchEnter {
        shift: bool,
    },
    SearchEnterChecked {
        shift: bool,
        in_find_field: bool,
    },
    /// The autosave wrote (or failed to write) the session.
    SessionWritten(Result<(), String>),
    /// Quit although the session couldn't be saved.
    QuitAnyway,
    /// Re-parses the preview if the text changed (typing is debounced).
    PreviewTick,
    /// The window was resized (kept for the next start).
    WindowResized(f32, f32),
    /// The window gained or lost the focus.
    WindowFocus(bool),
    // Overlays
    CloseOverlay,
    OpenGotoLine,
    GotoLineInputChanged(String),
    GotoLineSubmit,
    OpenLanguagePicker,
    SetLanguage(String),
    OpenShortcuts,
    /// A page of the project in the browser (documentation, issues).
    OpenUrl(&'static str),
    StartCaptureShortcut(ShortcutTarget),
    // Task palette
    OpenPalette,
    /// Search everywhere: opened on a tab, Shift twice, what is typed,
    /// another tab, a line opened, what the searches found, the options.
    OpenEverywhere(crate::everywhere::Tab),
    DoubleShift,
    EverywhereQuery(String),
    EverywhereTab(crate::everywhere::Tab),
    EverywhereOpen(Option<usize>),
    EverywhereFiles(u64, Vec<(PathBuf, String)>),
    EverywhereSymbolsDue(u64),
    EverywhereSymbols(u64, crate::lsp::Symbols),
    EverywhereTextDue(u64),
    EverywhereText(u64, Result<Vec<crate::finder::TextHit>, String>),
    EverywhereNonProject,
    EverywhereFilter,
    EverywhereFilterType(String, bool),
    EverywhereTextOption(u8),
    EverywhereExport,
    EverywhereAbbreviate(crate::palette::PaletteCommand),
    EverywhereAbbreviationTyped(String),
    EverywhereAbbreviationSubmit,
    EverywhereAbbreviationRemove(crate::palette::PaletteCommand, String),
    EverywhereAssign(crate::palette::PaletteCommand),
    // Line-level editing
    DuplicateLine,
    MoveLineUp,
    MoveLineDown,
    ToggleComment,
    DeleteLine,
    // Bookmarks
    ToggleBookmark,
    NextBookmark,
    PrevBookmark,
    ClearBookmarks,
    NavBack,
    NavForward,
    ToggleFold,
    FoldAll,
    UnfoldAll,
    /// A click on a chevron of the gutter (line of the block, how far it reaches).
    FoldClick(usize, crate::folding::Scope),
    /// Ctrl/Cmd+click on a name: look for where it is defined.
    GotoDefinition,
    /// Where the type of the name under the cursor is declared.
    GotoTypeDefinition,
    /// The imports of the file, put in order by its language server.
    OrganizeImports,
    /// What implements the name under the cursor.
    GotoImplementation,
    /// A line of the list of references was chosen.
    OpenReference(usize),
    /// Installs a language server (its program name) from the Preferences.
    InstallServer(&'static str),
    UninstallServer(&'static str),
    ServerUninstalled(&'static str, Result<(), String>),
    ServerInstalled(&'static str, Result<(), String>),
    /// The language servers that can be run.
    ServersChecked(Servers),
    /// The name looked for, and where it is defined, if anywhere.
    DefinitionFound(String, crate::lsp::Outcome),
    /// A right click over the text, relative to the editor.
    EditorRightClick(iced::Point),
    IndentSelection,
    DedentLine,
    // Recent files
    OpenRecentFile(PathBuf),
    /// The submenu that is open beside the menu (none: none).
    SetSubmenu(Option<crate::menus::SubMenu>),
    ClearRecents,
    /// Whether a search field had focus when an editor shortcut was pressed.
    ShortcutChecked(Box<Message>, bool),
    /// The user confirmed closing a tab that has unsaved changes.
    ConfirmDiscard(PathBuf),
    /// The user chose to overwrite a file that changed on disk.
    ConfirmOverwrite,
    // Updater
    CheckForUpdate,
    UpdateChecked(Result<Option<(String, Vec<crate::changelog::Entry>)>, String>),
    OpenChangelog,
    ConfirmInstallUpdate(String),
    UpdateDownloaded(Result<(std::path::PathBuf, String), String>),
    UpdateInstalled(Result<(), String>),
    // Markdown / JSON preview
    SetViewMode(ViewMode),
    JsonToggle(String),
    JsonExpandAll,
    JsonCollapseAll,
    /// Live modifier state, so a chevron click can tell Ctrl+click from Alt+click.
    ModifiersChanged {
        ctrl: bool,
        alt: bool,
        shift: bool,
    },
    /// An in-document anchor link (`#slug`) was clicked in the Markdown
    /// preview.
    MarkdownLinkClicked(String),
    /// Resolves the position of the heading `MarkdownLinkClicked` targeted.
    PreviewScrollTo(Option<f32>),
}

/// What the window starts with.
#[derive(Default)]
pub struct Flags {
    pub config: Config,
    /// Files named on the command line.
    pub files: Vec<crate::ipc::Target>,
}

/// The Preferences window: its size at first, the least it can be made, and
/// the width of the edges and the corner that resize it.
const PREFS_SIZE: (f32, f32) = (900.0, 640.0);
const PREFS_MIN: (f32, f32) = (520.0, 360.0);
const PREFS_EDGE: f32 = 6.0;
const PREFS_CORNER: f32 = 16.0;
/// What is left around it in the window at the first size.
const PREFS_MARGIN: f32 = 40.0;

/// The grip of the Preferences window being dragged: what it took hold of,
/// where the pointer was at the first move, and the window as it was.
#[derive(Debug, Clone, Copy)]
struct PrefsDrag {
    grab: crate::preferences::PrefsGrab,
    from: Option<iced::Point>,
    rect: (f32, f32, f32, f32),
}

pub struct ArdoiseApp {
    config: Config,
    /// Folders asked about trust in this run (once each).
    trust_asked: HashSet<PathBuf>,
    /// How many session writes were started (they land in this order).
    session_writes: u64,
    /// Tabs whose read was overtaken by another switch: their answer is dropped.
    abandoned_reads: HashSet<PathBuf>,
    /// Marked lines of every file, saved between runs.
    bookmarks: crate::bookmarks::Bookmarks,
    editor: EditorState,
    /// How the files stand (Git, lint), for their colors.
    file_marks: crate::projects::Marks,
    /// What "go to file" and "find in files" look into and found.
    finder: finder::FinderState,
    /// The folders added as projects, shown as trees in the sidebar.
    projects: crate::projects::Projects,
    lint: lint::LintState,
    completion: completion::CompletionState,
    /// The editor split in two panes, if it is.
    split: Option<split::Split>,
    sidebar: SidebarState,
    search: SearchState,
    preferences: PreferencesState,
    show_search: bool,
    show_sidebar: bool,
    /// The open files from the one seen last, and, while Ctrl+Tab is held,
    /// that order frozen and how far into it the switch went.
    mru: Vec<PathBuf>,
    tab_switch: Option<(Vec<PathBuf>, usize)>,
    /// The tab a file of the tree opened in, to be replaced by the next one
    /// (it is kept once it is edited or opened twice), the one it replaces
    /// with its place, and the last click in the tree.
    preview_tab: Option<PathBuf>,
    preview_replaces: Option<(PathBuf, usize)>,
    tree_click: Option<(PathBuf, std::time::Instant)>,
    /// The arrows move in the tree (it was clicked last).
    tree_focus: bool,
    /// The line pressed in the tree, and whether it was dragged since.
    tree_drag: Option<(PathBuf, bool)>,
    /// How far the tree is scrolled and how high it is.
    tree_view: (f32, f32),
    /// The tabs closed, last one last (Ctrl+Shift+T opens them again).
    closed_tabs: Vec<PathBuf>,
    /// The text of the untitled tabs that were closed with some in them.
    closed_untitled: Vec<String>,
    /// The list of every tab is open, from where its button was pressed.
    tab_list: Option<iced::Point>,
    /// The panels of the sidebar that are open (the bar on its left).
    panels: crate::sidebar::Panels,
    /// Width of the file list, and whether its edge is being dragged.
    sidebar_width: f32,
    /// The panel of a plugin at the right: its width, and its edge dragged.
    right_width: f32,
    right_resizing: bool,
    /// The API panel: collections, environments, history.
    sidebar_resizing: bool,
    /// The height of the bottom panels (Git, terminal, scratch files, bookmarks), and the drag that resizes it: where the
    /// pointer and the height were at its first move.
    panel_height: f32,
    panel_resizing: bool,
    panel_drag: Option<(f32, f32)>,
    show_preferences: bool,
    /// Where the Preferences window was put (its top left) and how big it was
    /// made; none: the middle, at its first size. And the grip being dragged.
    prefs_pos: Option<(f32, f32)>,
    prefs_size: Option<(f32, f32)>,
    prefs_drag: Option<PrefsDrag>,
    /// The shortcut being rebound, while waiting for its new key.
    capturing: Option<ShortcutTarget>,
    everywhere: everywhere::Everywhere,
    results: results::Results,
    /// The diff of a comparison.
    history_diff: crate::diff_view::Viewer,
    /// The groups of tabs that are folded.
    folded_tabs: BTreeSet<Option<PathBuf>>,
    open_menu: Option<TopMenu>,
    /// The submenu showing beside the open menu (the recent files, a line…).
    open_submenu: Option<crate::menus::SubMenu>,
    /// What the macOS menu bar was last built from.
    #[cfg(all(target_os = "macos", not(test)))]
    native_menu_state: String,
    /// Last known screen position of each menu button. Reused while a fresh
    /// query is in flight so switching menus doesn't flicker.
    menu_bounds: HashMap<TopMenu, iced::Rectangle>,
    current_file: Option<PathBuf>,
    is_dirty: bool,
    is_formatting: bool,
    status_message: String,
    scratch: crate::scratch::Panel,
    plugins: crate::plugins::Plugins,
    bookmark_panel: bookmark_panel::Panel,
    diff_tabs: HashMap<PathBuf, DiffTabState>,
    diff_counter: u32,
    /// What files held before a rollback or an arrow of a diff changed them
    /// (Ctrl+Z in a diff brings it back), and what Ctrl+Y puts again.
    work_undo: Vec<(PathBuf, Vec<u8>)>,
    work_redo: Vec<(PathBuf, Vec<u8>)>,
    merge_tabs: MergeTabs,
    merge_counter: u32,
    blame: Option<BlameState>,
    marks: Option<MarksState>,
    /// The file the marks were last asked for.
    marks_asked: Option<PathBuf>,
    /// Typing in the result of a merge (the app's shortcuts wait).
    merge_typing: bool,
    last_edit: std::time::Instant,
    /// A save by itself is under way (it says nothing in the status bar).
    /// How many saves of an auto-save batch are still on their way.
    autosaving: usize,
    ticks: u32,
    /// Whether the window has the focus: the periodic checks of the disk
    /// wait while it has not.
    window_focused: bool,
    /// The disk is watched for changes.
    watch: crate::watch::Watch,
    /// Something changed on the disk since the tree and the marks were read.
    fs_dirty: bool,
    /// The tree of the projects is being read in the background.
    projects_reloading: bool,
    /// The text of the buffer as of a stamp (`buffer_text_shared`).
    text_shared: std::cell::RefCell<Option<SharedText>>,
    /// When the marks of the changed lines were last computed while typing.
    marks_at: std::time::Instant,
    /// The last right click on the editor was in the gutter.
    gutter_click: bool,
    /// The line of the gutter that was right-clicked (for the blame menu).
    gutter_line: Option<usize>,
    /// The change whose revert button is open: file, line, and the buffer it was
    /// opened on (typing closes it).
    revert: Option<(PathBuf, usize, (u64, u64))>,
    /// A language server to offer in the status bar, in place of the message.
    server_offer: Option<&'static crate::lsp_install::ServerInfo>,
    status_is_error: bool,
    format_error: Option<String>,
    show_error_panel: bool,
    untitled_counter: u32,
    file_cache: HashMap<PathBuf, CachedBuffer>,
    overlay: ActiveOverlay,
    goto_line_input: String,
    /// Word-level undo history (capped by count and total size).
    undo_stack: History,
    /// Redo history rebuilt when undoing.
    redo_stack: History,
    /// Undo/redo history of the tabs that aren't the active one.
    undo_cache: HashMap<PathBuf, (History, History)>,
    /// Preference changes are written out on the next autosave tick, not on
    /// every slider movement.
    config_dirty: bool,
    /// Fingerprint of the last session JSON written, so unchanged sessions
    /// aren't rewritten.
    /// The fingerprint of the session last written (shared with the job that
    /// writes it).
    last_session: std::sync::Arc<std::sync::atomic::AtomicU64>,
    /// Held while this is the window that saves the session.
    session_lock: Option<crate::session::InstanceLock>,
    /// Whether this window saves the session and the bookmarks.
    session_owner: bool,
    /// Bumped by every message that may change what the session holds, so the
    /// autosave tick can skip the work when nothing happened.
    rev: u64,
    /// `rev` the session was last built at.
    session_rev: u64,
    /// How the current file stores its text (line endings, final newline).
    format: TextFormat,
    /// How each file is indented: what it was found to be, or what was
    /// chosen (spaces or tabs, columns).
    indents: HashMap<PathBuf, (bool, usize)>,
    /// The files too large to open whole, in the part being read.
    large: HashMap<PathBuf, LargeView>,
    /// The row of the list of the window in front that the keys are on.
    overlay_pick: usize,
    /// Which window `overlay_pick` was for.
    overlay_seen: Option<std::mem::Discriminant<ActiveOverlay>>,
    /// Saving waits for the formatting that was started for it.
    format_on_save_pending: bool,
    /// The save that follows the formatting does not start another.
    skip_format_once: bool,
    /// What the language server says of the name under the pointer, and
    /// where the card goes.
    symbol_card: Option<(iced::Point, String)>,
    /// Changes with the pointer and the typing: an answer from before is stale.
    symbol_token: u64,
    /// Other commands hand their files to this window.
    ipc_listening: bool,
    spell: spelling::SpellState,
    /// The local copies of remote files, where they came from and the date
    /// they had there.
    /// State on disk of the current file when it was last read or written,
    /// to notice edits made by other programs.
    disk_stamp: Option<DiskStamp>,
    /// Fingerprint of the text last read or saved, to see that an undo got
    /// back to it.
    saved_fp: Option<u64>,
    /// The tab the user last asked to switch to, so a slow read of another
    /// one doesn't take over.
    pending_open: Option<PathBuf>,
    /// Where to put the cursor (line, byte, length) once that file is the
    /// active tab: the definition being jumped to.
    pending_jump: Option<(PathBuf, usize, usize, usize)>,
    /// What the Preferences show about language servers.
    servers: crate::preferences::ServerStatus,
    /// Places left by jumps (definitions, go to line, bookmarks): Back and
    /// Forward walk through them.
    nav_back: Vec<Place>,
    nav_forward: Vec<Place>,
    /// Whether the "changed on disk" warning was already shown.
    external_warned: bool,
    /// When the part of a large file in front was last read again.
    large_refreshed: Option<std::time::Instant>,
    /// Size on disk of `size_for`, cached so `view` does no I/O.
    file_size: Option<u64>,
    size_for: Option<PathBuf>,
    /// Whether the next edit should open a new undo group.
    /// True at start, after whitespace, after a cursor move, after delete.
    undo_new_group: bool,
    /// When an undo step was last made (steps are merged, on a big file, when
    /// they come in the same second).
    last_undo_at: Option<std::time::Instant>,
    /// The kind of deletion (Backspace or Delete) the open undo group is
    /// made of, so that the next one of the same kind joins it.
    undo_deletion: Option<std::mem::Discriminant<text_editor::Edit>>,
    /// How the editor pane renders the buffer (Markdown / JSON files only).
    view_mode: ViewMode,
    /// Parsed preview of the buffer, rebuilt only when the text changes.
    preview: PreviewDoc,
    /// The status-bar label of the selection (`None` without one), kept up to
    /// date by `update` so `view` doesn't copy a possibly huge selection on
    /// every frame.
    selection_info: Option<String>,
    /// What `selection_info` was computed for (selection, text revision).
    selection_key: Option<(Option<crate::code_editor::SelectionRange>, u64)>,
    /// `text_rev` the preview was parsed at.
    preview_text_rev: u64,
    /// Bumped by every message that may change the buffer text.
    text_rev: u64,
    /// Folded node paths of the JSON tree.
    json_collapsed: std::sync::Arc<HashSet<String>>,
    /// Bumped whenever what the preview shows changes, so the cached widget
    /// tree (iced `lazy`) is rebuilt only then, not on every frame.
    preview_rev: u64,
    /// Closers typed by the auto-pairing and not stepped over yet:
    /// (line, byte column, character).
    pending_closers: Vec<(usize, usize, char)>,
    /// Ctrl/Cmd currently held, tracked for Ctrl+click on a chevron.
    mod_ctrl: bool,
    /// Alt currently held, tracked for Alt+click on a chevron.
    mod_alt: bool,
    mod_shift: bool,
}

/// Cap on the memory the undo history of one buffer may hold.
const UNDO_MAX_BYTES: usize = 64 * 1024 * 1024;
/// Cap on the undo history of all tabs together.
const UNDO_TOTAL_BYTES: usize = 256 * 1024 * 1024;
const UNDO_MAX_STEPS: usize = 100;
/// Files larger than this are refused instead of freezing the editor.
const MAX_OPEN_BYTES: u64 = 64 * 1024 * 1024;
/// A file larger than that is shown in parts of this size, read only.
const LARGE_WINDOW: u64 = 8 * 1024 * 1024;

/// The part of a large file in the editor: the bytes it covers and the size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LargeView {
    pub start: u64,
    pub end: u64,
    pub size: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LargeMove {
    First,
    Previous,
    Next,
    Last,
}

fn megabytes(bytes: u64) -> String {
    if bytes >= 1024 * 1024 * 1024 {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}
/// The same limit for pasted text.
/// A line longer than this makes layout and typing slow.
const LONG_LINE_BYTES: usize = 20_000;
const MAX_PASTE_BYTES: usize = 64 * 1024 * 1024;
/// Marker error for a dialog the user dismissed: not worth a status message.
const CANCELLED: &str = "cancelled";

/// What a background format run started from: the format result is only
/// applied if the buffer still reads the same.
#[derive(Debug, Clone)]
pub struct FormatJob {
    path: Option<PathBuf>,
    /// Fingerprint of the text that was sent (the whole buffer, or the
    /// selection).
    source: u64,
}

fn trim_history(stack: &mut History) {
    stack.trim(UNDO_MAX_STEPS, UNDO_MAX_BYTES);
}

fn history_bytes(stacks: &(History, History)) -> usize {
    stacks.0.bytes() + stacks.1.bytes()
}

/// The cached parse backing the preview pane.
enum PreviewDoc {
    None,
    Markdown(std::sync::Arc<Vec<crate::preview::markdown::Block>>),
    Json(std::sync::Arc<serde_json::Value>),
    JsonError(String),
}

impl ArdoiseApp {
    /// An app with one empty untitled tab, reading nothing from disk.
    fn blank(config: Config) -> Self {
        crate::trust::set_all(&config.trusted_folders);
        crate::lsp::set_completion_options(&config.completion_options);
        Self {
            bookmarks: crate::bookmarks::Bookmarks::default(),
            editor: EditorState::new(),
            file_marks: crate::projects::Marks::default(),
            finder: finder::FinderState::default(),
            projects: crate::projects::Projects::new(
                config.projects.clone(),
                config.projects_open.clone(),
                config.projects_hide_ignored,
                config.projects_exclude.clone(),
            ),
            lint: lint::LintState::default(),
            completion: completion::CompletionState::default(),
            split: None,
            sidebar: SidebarState::new(),
            search: SearchState::new(),
            preferences: PreferencesState::from_config(&config),
            show_search: false,
            show_sidebar: config.show_sidebar,
            // One panel at a time: with both asked for, the projects win once
            // there are some.
            mru: Vec::new(),
            tab_switch: None,
            closed_tabs: Vec::new(),
            closed_untitled: Vec::new(),
            preview_tab: None,
            preview_replaces: None,
            tree_click: None,
            tree_focus: false,
            tree_drag: None,
            tree_view: (0.0, 0.0),
            tab_list: None,
            panels: crate::sidebar::Panels {
                files: config.show_files_panel
                    && (!config.show_projects_panel || config.projects.is_empty()),
                projects: config.show_projects_panel
                    && (!config.show_files_panel || !config.projects.is_empty()),
            },
            trust_asked: HashSet::new(),
            session_writes: 0,
            abandoned_reads: HashSet::new(),
            sidebar_width: config.sidebar_width.clamp(SIDEBAR_MIN, SIDEBAR_MAX),
            right_width: config.right_panel_width.clamp(RIGHT_MIN, RIGHT_MAX),
            right_resizing: false,
            sidebar_resizing: false,
            panel_height: PANEL_HEIGHT,
            panel_resizing: false,
            panel_drag: None,
            show_preferences: false,
            prefs_pos: None,
            prefs_size: None,
            prefs_drag: None,
            capturing: None,
            everywhere: everywhere::Everywhere::default(),
            results: results::Results::default(),
            history_diff: Default::default(),
            folded_tabs: BTreeSet::new(),
            open_menu: None,
            open_submenu: None,
            #[cfg(all(target_os = "macos", not(test)))]
            native_menu_state: String::new(),
            menu_bounds: HashMap::new(),
            current_file: None,
            is_dirty: false,
            is_formatting: false,
            status_message: t!("status.ready").to_string(),
            scratch: crate::scratch::Panel::new(&config.scratch_dir),
            plugins: Default::default(),
            bookmark_panel: Default::default(),
            diff_tabs: HashMap::new(),
            diff_counter: 0,
            work_undo: Vec::new(),
            work_redo: Vec::new(),
            merge_tabs: HashMap::new(),
            merge_counter: 0,
            blame: None,
            marks: None,
            marks_asked: None,
            merge_typing: false,
            last_edit: std::time::Instant::now(),
            autosaving: 0,
            ticks: 0,
            window_focused: true,
            watch: crate::watch::Watch::default(),
            fs_dirty: true,
            projects_reloading: false,
            text_shared: Default::default(),
            marks_at: std::time::Instant::now(),
            gutter_click: false,
            gutter_line: None,
            revert: None,
            server_offer: None,
            status_is_error: false,
            format_error: None,
            show_error_panel: false,
            untitled_counter: 1,
            file_cache: HashMap::new(),
            overlay: ActiveOverlay::None,
            goto_line_input: String::new(),
            undo_stack: History::default(),
            redo_stack: History::default(),
            undo_cache: HashMap::new(),
            config_dirty: false,
            last_session: Default::default(),
            session_lock: None,
            session_owner: true,
            rev: 1,
            session_rev: 0,
            format: TextFormat::default(),
            indents: HashMap::new(),
            large: HashMap::new(),
            overlay_pick: 0,
            overlay_seen: None,
            format_on_save_pending: false,
            skip_format_once: false,
            symbol_card: None,
            symbol_token: 0,
            ipc_listening: false,
            spell: Default::default(),
            disk_stamp: None,
            saved_fp: None,
            pending_open: None,
            pending_jump: None,
            servers: Default::default(),
            nav_back: Vec::new(),
            nav_forward: Vec::new(),
            external_warned: false,
            large_refreshed: None,
            file_size: None,
            size_for: None,
            undo_new_group: true,
            last_undo_at: None,
            undo_deletion: None,
            view_mode: ViewMode::Raw,
            preview: PreviewDoc::None,
            selection_info: None,
            selection_key: None,
            preview_text_rev: 0,
            text_rev: 1,
            json_collapsed: Default::default(),
            preview_rev: 0,
            pending_closers: Vec::new(),
            mod_ctrl: false,
            mod_alt: false,
            mod_shift: false,
            config,
        }
    }
}

impl ArdoiseApp {
    pub fn new(flags: Flags) -> (Self, Task<Message>) {
        // The first window owns the session; others start empty and leave it
        // alone, so closing them can't overwrite it.
        let lock = crate::session::InstanceLock::acquire();
        let owner = lock.is_some();
        let loaded = if owner {
            crate::session::Session::load()
        } else {
            crate::session::Loaded {
                session: crate::session::Session::default(),
                set_aside: None,
            }
        };
        let mut app = Self::blank(flags.config);
        app.session_lock = lock;
        app.session_owner = owner;
        if owner {
            app.bookmarks = crate::bookmarks::Bookmarks::load();
        } else {
            app.set_status(t!("status.second_window"), false);
        }

        app.bookmarks.prune_missing();
        // The plugins first: the session brings back their tabs.
        if owner {
            crate::plugins::migrate::carry_lint_settings(&app.config);
            crate::plugins::migrate::carry_personal_dictionary();
            crate::plugins::migrate::carry_local_history(&app.config);
            crate::plugins::migrate::carry_api(&app.config);
            crate::plugins::migrate::carry_git_settings(&app.config);
            crate::plugins::migrate::carry_database();
        }
        app.plugins = crate::plugins::Plugins::load();
        app.plugins_start();
        let mut commands = vec![app.restore_session(loaded.session)];
        // The tools the user had before the plugins come back as plugins.
        if owner {
            commands.push(app.plugins_carry());
        }
        if let Some(aside) = loaded.set_aside {
            let name = aside
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            app.set_status(t!("status.session_set_aside", name = name.as_str()), true);
        }
        if let Some(name) = app.config.set_aside.clone() {
            app.set_status(t!("status.config_set_aside", name = name.as_str()), true);
        }
        commands.push(app.announce_new_version());

        // This window takes the files that other commands name.
        app.ipc_listening = owner && crate::ipc::listen();
        // Files given on the command line open on top of the session.
        for target in flags.files {
            commands.push(app.open_target(target));
        }
        (app, Task::batch(commands))
    }

    pub fn title(&self) -> String {
        let name = self
            .current_file
            .as_deref()
            .map(file_label)
            .unwrap_or_else(|| t!("app.untitled").to_string());
        let dirty = if self.is_dirty { " •" } else { "" };
        format!("Ardoise — {}{}", name, dirty)
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        let task = self.update_message(message);
        self.plugins_follow_settings();
        self.plugins_follow_active();
        self.plugins_follow_workspace();
        self.plugins_follow_tab();
        // The tests run off the main thread, where the system menu is off limits.
        #[cfg(all(target_os = "macos", not(test)))]
        self.sync_native_menu();
        task
    }

    /// Rebuilds the macOS menu bar when what it shows has changed.
    #[cfg(all(target_os = "macos", not(test)))]
    fn sync_native_menu(&mut self) {
        let has_formatter = self
            .editor
            .language
            .as_deref()
            .map(crate::formatter::can_format)
            .unwrap_or(false);
        let state = crate::native_menu::State {
            config: &self.config,
            has_formatter,
            lang_picker_enabled: self.language_picker_enabled(),
            bookmarks: self.bookmark_state(),
            tools: self.plugins.tools(),
            split: self.split.is_some(),
            panels: self.menu_panels(),
            editor: self.menu_editor(),
        };
        let signature = state.signature();
        if signature != self.native_menu_state {
            crate::native_menu::install(&state);
            self.native_menu_state = signature;
        }
    }

    fn update_message(&mut self, message: Message) -> Task<Message> {
        // What the autosave and the preview have to look at again.
        let passive = message.is_background()
            || matches!(
                &message,
                Message::AutoSave
                    | Message::ModifiersChanged { .. }
                    | Message::MenuBounds(..)
                    | Message::ToggleMenu(_)
                    | Message::HoverMenu(_)
                    | Message::SetSubmenu(_)
                    | Message::SessionWritten(_)
                    | Message::PreviewTick
                    | Message::WindowResized(..)
                    | Message::WindowFocus(_)
                    | Message::DiffTab(DiffNav::Pick(_) | DiffNav::Scrolled(..))
            );
        let edits_text = !passive && !matches!(&message, Message::EditorAction(a) if !a.is_edit());
        // The preview and the split copies follow the text only when it
        // really changed (see below).
        let stamp_before = self.buffer_stamp();
        if !passive {
            self.rev += 1;
        }
        // While typing in a preview, the parse waits for the next tick.
        let typing = matches!(&message, Message::EditorAction(a) if a.is_edit());
        if edits_text {
            self.last_edit = std::time::Instant::now();
        }
        // The terminal redraws 30 times a second: it changes nothing the rest
        // of the window follows (the text, the preview, the lint, the marks).
        let idle = false;
        let command = self.handle(message);
        if idle {
            return command;
        }
        if edits_text {
            self.snippet_follow();
        }
        if self
            .completion
            .session
            .as_ref()
            .is_some_and(|s| self.current_file.as_ref() != Some(&s.path))
        {
            self.completion_close();
        }
        if self
            .completion
            .hint
            .as_ref()
            .is_some_and(|h| self.current_file.as_ref() != Some(h.path()))
        {
            self.completion.hint = None;
        }
        let command = if passive {
            command
        } else {
            Task::batch([command, self.completion_prepare()])
        };
        if edits_text && self.buffer_stamp() != stamp_before {
            self.text_rev += 1;
        }
        self.sync_split();
        let marks_task = self.after_message(edits_text);
        self.track_mru();
        let preview_task = self.settle_preview();
        let lint_task = self.lint_tick();
        let command = Task::batch([command, marks_task, lint_task, preview_task]);
        if !typing {
            self.refresh_preview();
        }
        self.refresh_file_size();
        let command = Task::batch([
            command,
            self.refresh_bookmark_panel(),
            self.finder_preview_task(),
        ]);
        if !passive {
            self.refresh_selection_info();
        }
        command
    }

    pub fn subscription(&self) -> iced::Subscription<Message> {
        // `listen_with` also sees events that a widget captured: the search
        // field swallows every key, the editor all arrows and Enter, and the
        // shortcuts must still work then.
        let keys = iced::event::listen_with(|event, status, _window| {
            use iced::event::Status;
            use iced::keyboard::{key::Named, Event, Key};

            let iced::Event::Keyboard(Event::KeyPressed { key, modifiers, .. }) = event else {
                return None;
            };
            if key == Key::Named(Named::Escape) {
                return Some(Message::EscapePressed);
            }

            let key_str = match key.as_ref() {
                Key::Character(c) => c.to_lowercase(),
                Key::Named(Named::Enter) => "Return".to_string(),
                Key::Named(Named::Tab) => "Tab".to_string(),
                Key::Named(Named::ArrowUp) => "ArrowUp".to_string(),
                Key::Named(Named::ArrowDown) => "ArrowDown".to_string(),
                Key::Named(Named::ArrowLeft) => "ArrowLeft".to_string(),
                Key::Named(Named::ArrowRight) => "ArrowRight".to_string(),
                Key::Named(Named::Home) => "Home".to_string(),
                Key::Named(Named::End) => "End".to_string(),
                Key::Named(Named::PageUp) => "PageUp".to_string(),
                Key::Named(Named::PageDown) => "PageDown".to_string(),
                Key::Named(Named::Delete) => "Delete".to_string(),
                Key::Named(Named::Insert) => "Insert".to_string(),
                Key::Named(Named::Backspace) => "Backspace".to_string(),
                Key::Named(Named::Space) => "Space".to_string(),
                Key::Named(Named::F1) => "F1".to_string(),
                Key::Named(Named::F2) => "F2".to_string(),
                Key::Named(Named::F3) => "F3".to_string(),
                Key::Named(Named::F4) => "F4".to_string(),
                Key::Named(Named::F5) => "F5".to_string(),
                Key::Named(Named::F6) => "F6".to_string(),
                Key::Named(Named::F7) => "F7".to_string(),
                Key::Named(Named::F8) => "F8".to_string(),
                Key::Named(Named::F9) => "F9".to_string(),
                Key::Named(Named::F10) => "F10".to_string(),
                Key::Named(Named::F11) => "F11".to_string(),
                Key::Named(Named::F12) => "F12".to_string(),
                _ => return None,
            };
            // Plain typing in a widget is none of the app's business (and
            // would wake it up on every keystroke); modified keys and keys a
            // widget left alone are.
            let ignored = status == Status::Ignored;
            let is_enter = key_str == "Return";
            if !(ignored || modifiers.command() || modifiers.alt() || is_enter) {
                return None;
            }

            Some(Message::KeyPressed {
                key: key_str,
                ctrl: modifiers.command(),
                shift: modifiers.shift(),
                alt: modifiers.alt(),
            })
        });

        // Shift twice: search everywhere.
        let double_shift = iced::event::listen_with(|event, _status, _window| match event {
            iced::Event::Keyboard(key)
                if everywhere::double_shift::hear(&key, std::time::Instant::now()) =>
            {
                Some(Message::DoubleShift)
            }
            _ => None,
        });

        // Chevron clicks need the modifier state at click time, which a button
        // press message does not carry.
        let modifiers = iced::event::listen_with(|event, _status, _window| match event {
            iced::Event::Keyboard(iced::keyboard::Event::ModifiersChanged(m)) => {
                Some(Message::ModifiersChanged {
                    ctrl: m.command(),
                    alt: m.alt(),
                    shift: m.shift(),
                })
            }
            // The window is closed through the app so the session is saved.
            iced::Event::Window(iced::window::Event::CloseRequested) => Some(Message::Quit),
            iced::Event::Window(iced::window::Event::Focused) => Some(Message::WindowFocus(true)),
            iced::Event::Window(iced::window::Event::Unfocused) => {
                Some(Message::WindowFocus(false))
            }
            iced::Event::Window(iced::window::Event::Resized(size)) => {
                Some(Message::WindowResized(size.width, size.height))
            }
            // The back / forward buttons of a mouse.
            iced::Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Back)) => {
                Some(Message::NavBack)
            }
            iced::Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Forward)) => {
                Some(Message::NavForward)
            }
            _ => None,
        });

        let autosave =
            iced::time::every(std::time::Duration::from_secs(1)).map(|_| Message::AutoSave);

        let mut subscriptions = vec![
            keys,
            double_shift,
            modifiers,
            autosave,
            crate::watch::changes().map(|()| Message::FsChanged),
        ];
        // Enter, when no field took it (a row was clicked): opens the match.
        if self.overlay == ActiveOverlay::FindInFiles {
            subscriptions.push(iced::event::listen_with(|event, status, _window| {
                use iced::keyboard::{key::Named, Event, Key};
                match event {
                    iced::Event::Keyboard(Event::KeyPressed {
                        key: Key::Named(Named::Enter),
                        modifiers,
                        ..
                    }) if status == iced::event::Status::Ignored && modifiers.is_empty() => {
                        Some(Message::FindOpen(None))
                    }
                    _ => None,
                }
            }));
        }
        if self.ipc_listening {
            subscriptions.push(crate::ipc::arrivals().map(|()| Message::IpcPoll));
        }
        if self.prefs_drag.is_some() {
            subscriptions.push(iced::event::listen_with(
                |event, _status, _window| match event {
                    iced::Event::Mouse(iced::mouse::Event::CursorMoved { position }) => {
                        Some(Message::PrefsDragMove(position))
                    }
                    iced::Event::Mouse(iced::mouse::Event::ButtonReleased(
                        iced::mouse::Button::Left,
                    )) => Some(Message::PrefsDragEnd),
                    _ => None,
                },
            ));
        }
        if self.sidebar_resizing {
            subscriptions.push(iced::event::listen_with(
                |event, _status, _window| match event {
                    iced::Event::Mouse(iced::mouse::Event::CursorMoved { position }) => {
                        Some(Message::SidebarResize(position.x))
                    }
                    iced::Event::Mouse(iced::mouse::Event::ButtonReleased(
                        iced::mouse::Button::Left,
                    )) => Some(Message::SidebarResizeEnd),
                    _ => None,
                },
            ));
        }
        if self.tree_drag.is_some() {
            subscriptions.push(iced::event::listen_with(
                |event, _status, _window| match event {
                    iced::Event::Mouse(iced::mouse::Event::ButtonReleased(
                        iced::mouse::Button::Left,
                    )) => Some(Message::TreeRelease),
                    _ => None,
                },
            ));
        }
        if self.plugin_grid_resizing() {
            subscriptions.push(iced::event::listen_with(
                |event, _status, _window| match event {
                    iced::Event::Mouse(iced::mouse::Event::CursorMoved { position }) => Some(
                        Message::Plugin(crate::plugins::PluginMsg::GridResize(position.x)),
                    ),
                    iced::Event::Mouse(iced::mouse::Event::ButtonReleased(
                        iced::mouse::Button::Left,
                    )) => Some(Message::Plugin(crate::plugins::PluginMsg::GridResizeEnd)),
                    _ => None,
                },
            ));
        }
        if self.right_resizing {
            subscriptions.push(iced::event::listen_with(
                |event, _status, _window| match event {
                    iced::Event::Mouse(iced::mouse::Event::CursorMoved { position }) => Some(
                        Message::Plugin(crate::plugins::PluginMsg::RightResize(position.x)),
                    ),
                    iced::Event::Mouse(iced::mouse::Event::ButtonReleased(
                        iced::mouse::Button::Left,
                    )) => Some(Message::Plugin(crate::plugins::PluginMsg::RightResizeEnd)),
                    _ => None,
                },
            ));
        }
        if self.panel_resizing {
            subscriptions.push(iced::event::listen_with(
                |event, _status, _window| match event {
                    iced::Event::Mouse(iced::mouse::Event::CursorMoved { position }) => {
                        Some(Message::PanelResize(position.y))
                    }
                    iced::Event::Mouse(iced::mouse::Event::ButtonReleased(
                        iced::mouse::Button::Left,
                    )) => Some(Message::PanelResizeEnd),
                    _ => None,
                },
            ));
        }
        if self.view_mode != ViewMode::Raw && self.preview_kind().is_some() {
            subscriptions.push(
                iced::time::every(std::time::Duration::from_millis(150))
                    .map(|_| Message::PreviewTick),
            );
        }
        subscriptions.push(crate::plugins::host::events());
        #[cfg(all(target_os = "macos", not(test)))]
        subscriptions.push(crate::native_menu::subscription());
        iced::Subscription::batch(subscriptions)
    }

    /// The Preferences window as it is shown: `(x, y, width, height)`. It
    /// stays inside the window of the app, whatever that became since.
    fn prefs_rect(&self) -> (f32, f32, f32, f32) {
        let (ww, wh) = (self.config.window_width, self.config.window_height);
        let (w, h) = self.prefs_size.unwrap_or((
            PREFS_SIZE.0.min(ww - 2.0 * PREFS_MARGIN),
            PREFS_SIZE.1.min(wh - 2.0 * PREFS_MARGIN),
        ));
        let (w, h) = (w.max(PREFS_MIN.0).min(ww), h.max(PREFS_MIN.1).min(wh));
        let (x, y) = self.prefs_pos.unwrap_or(((ww - w) / 2.0, (wh - h) / 2.0));
        let x = x.min(ww - w).max(0.0);
        let y = y.min(wh - h).max(0.0);
        (x, y, w, h)
    }

    pub fn view(&self) -> Element<'_, Message> {
        let dark = self.config.dark_mode;
        se_theme::set_high_contrast(self.config.high_contrast);

        let title_bar = crate::menu_bar::view(&self.config, self.open_menu);

        let preview_kind = self.preview_kind();
        let view_mode = if preview_kind.is_some() {
            self.view_mode
        } else {
            ViewMode::Raw
        };
        let editor_with_context = self.editor_pane(preview_kind, view_mode, dark);
        let editor_with_context = self.with_split(editor_with_context, dark);

        // The search card floats over the top-right corner of the editor.
        let body: Element<'_, Message> = if self.show_search {
            crate::floating::top_right(
                editor_with_context,
                self.search.view(dark),
                12.0,
                // Clear of the minimap.
                16.0 + if self.config.show_minimap && view_mode != ViewMode::Preview {
                    crate::code_editor::MINIMAP_WIDTH
                } else {
                    0.0
                },
            )
        } else {
            editor_with_context
        };

        let (line, byte_col) = self.editor.content.cursor_position();
        // Columns count characters, not the bytes the editor indexes by.
        let cursor = (
            line,
            self.editor
                .content
                .line(line)
                .map_or(0, |l| l[..floor_boundary(&l, byte_col)].chars().count()),
        );
        let selection_info = self.selection_info.clone();
        let status_bar = crate::editor::statusbar::view(crate::editor::statusbar::Info {
            status: &self.status_message,
            is_error: self.status_is_error,
            offer: self.status_offer(),
            dark,
            cursor,
            line_ending: if self.format.mixed {
                t!("status.mixed_endings").to_string()
            } else {
                self.format.ending.label().to_string()
            },
            language: self.editor.language.as_deref(),
            selection: selection_info,
            file_size: self.file_size,
            use_spaces: self.indent().0,
            tab_width: self.indent().1,
            encoding: self.format.encoding.label(),
            problems: self.problem_counts(),
            problem_colors: (
                crate::lint::color(&self.config, crate::lint::Severity::Error),
                crate::lint::color(&self.config, crate::lint::Severity::Warning),
            ),
            lint_hint: self.lint_hint(),
            plugin_items: self.plugin_status_items(),
        });

        let error_panel: Option<Element<Message>> = if self.show_error_panel {
            self.format_error.as_ref().map(|err| {
                let header = row![
                    text(t!("panel.errors").to_string())
                        .size(10.5)
                        .font(se_theme::UI_MEDIUM)
                        .color(se_theme::muted_text(dark)),
                    Space::new().width(Length::Fill),
                    crate::theme::with_tip(
                        dark,
                        button(text("×").size(16).color(se_theme::muted_text(dark)))
                            .padding([2, 6])
                            .on_press(Message::CloseErrorPanel)
                            .style(
                                (se_theme::GhostButton {
                                    dark,
                                    active: false,
                                })
                                .style()
                            ),
                        t!("popin.close").to_string()
                    ),
                ]
                .padding([4, 14])
                .align_y(Alignment::Center);

                let body = scrollable(
                    container(
                        text(err.clone())
                            .size(12)
                            .color(se_theme::error_color(dark))
                            .font(se_theme::MONO),
                    )
                    .padding(iced::Padding {
                        top: 4.0,
                        right: 14.0,
                        bottom: 8.0,
                        left: 14.0,
                    })
                    .width(Length::Fill),
                )
                .style(se_theme::Scrollbar { dark }.style())
                .height(Length::Fixed(100.0));

                container(column![rule(dark), header, body])
                    .width(Length::Fill)
                    .style(se_theme::error_panel(dark))
                    .into()
            })
        } else {
            None
        };

        // The editor panel: toolbar, (formatting banner), body, (errors), status.
        let mut panel = column![
            self.tab_bar(dark),
            self.toolbar_view(preview_kind, view_mode, dark),
            rule(dark)
        ];
        if self.is_formatting {
            panel = panel.push(
                container(
                    text(t!("status.formatting").to_string())
                        .size(12)
                        .color(se_theme::accent_color(dark)),
                )
                .width(Length::Fill)
                .padding([4, 14])
                .style(se_theme::accent_banner(dark)),
            );
        }
        panel = panel.push(body);
        if let Some(errors) = error_panel {
            panel = panel.push(errors);
        }
        if self.scratch.open {
            panel = panel.push(self.scratch_panel(dark));
        }
        if self.bookmark_panel.open {
            panel = panel.push(self.bookmark_panel_view(dark));
        }
        if self.results.open && !self.results.tabs.is_empty() {
            panel = panel.push(self.results_view(dark));
        }
        if let Some(bottom) = self.plugin_bottom(dark) {
            panel = panel.push(bottom);
        }
        let editor_panel = container(panel.push(status_bar))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(se_theme::editor_panel(dark));

        // The panels touch, split by hairlines; the edge between the list
        // and the editor is the handle that resizes the list.
        let mut main_row = row![].height(Length::Fill);
        main_row = main_row.push(self.activity_bar(dark)).push(vrule(dark));
        let plugin_sidebar = self.plugin_sidebar(dark);
        if self.show_sidebar
            && (self.panels.files || self.panels.projects || plugin_sidebar.is_some())
        {
            let panel: Element<Message> = if let Some(panel) = plugin_sidebar {
                panel
            } else {
                self.sidebar.view(
                    dark,
                    &self.current_file,
                    &self.dirty_files(),
                    &self.bookmarks.marked_paths(),
                    &self.projects,
                    &self.config.project_colors,
                    &self.file_marks,
                    self.panels,
                    self.tree_view,
                    self.sidebar_width,
                    self.plugin_menus(),
                )
            };
            main_row = main_row.push(container(panel).width(Length::Fixed(self.sidebar_width)));
            // The edge between the list and the editor (a strip of the list
            // and the hairline): drag it to resize.
            main_row = main_row.push(
                iced::widget::mouse_area(
                    row![
                        container(Space::new().width(SIDEBAR_GRIP).height(Length::Fill))
                            .style(se_theme::panel(dark)),
                        vrule(dark),
                    ]
                    .height(Length::Fill),
                )
                .on_press(Message::SidebarResizeStart)
                .interaction(iced::mouse::Interaction::ResizingHorizontally),
            );
        }
        main_row = main_row.push(editor_panel);
        if let Some(panel) = self.plugin_right(dark) {
            // Its left edge is the handle that resizes it.
            main_row = main_row.push(
                iced::widget::mouse_area(
                    row![
                        vrule(dark),
                        container(Space::new().width(SIDEBAR_GRIP).height(Length::Fill))
                            .style(se_theme::panel(dark)),
                    ]
                    .height(Length::Fill),
                )
                .on_press(Message::Plugin(crate::plugins::PluginMsg::RightResizeStart))
                .interaction(iced::mouse::Interaction::ResizingHorizontally),
            );
            main_row = main_row.push(panel);
        }

        // On macOS the menus are in the system bar: no title bar here.
        let content: Element<Message> = if cfg!(target_os = "macos") {
            main_row.into()
        } else {
            column![title_bar, rule(dark), main_row].into()
        };
        let base: Element<Message> = container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(se_theme::window(dark))
            .into();

        // Floating dropdown (menu bar)
        // Bounds resolve one update cycle after a menu first opens (see
        // Message::ToggleMenu); until then, render nothing.
        // The layers below are always there, empty when nothing is open: the
        // tree of widgets must keep its shape, or the editor under them
        // loses its focus and its scroll each time one opens or closes.
        let (menu_outside, menu_dropdown, menu_at): (
            Element<Message>,
            Element<Message>,
            iced::Point,
        ) = if let (Some(menu), Some(bounds)) = (
            self.open_menu,
            self.open_menu
                .and_then(|m| self.menu_bounds.get(&m).copied()),
        ) {
            let has_fmt = self
                .editor
                .language
                .as_deref()
                .map(crate::formatter::can_format)
                .unwrap_or(false);
            let lang_override_enabled = self.language_picker_enabled();
            let dropdown = crate::menu_bar::dropdown_view(
                menu,
                &crate::menus::MenuState {
                    config: &self.config,
                    has_formatter: has_fmt,
                    lang_picker_enabled: lang_override_enabled,
                    bookmarks: self.bookmark_state(),
                    tools: self.plugins.tools(),
                    split: self.split.is_some(),
                    panels: self.menu_panels(),
                    editor: self.menu_editor(),
                },
                self.open_submenu,
            );
            // Under the dropdown and below the menu bar (its titles stay
            // clickable), anywhere else closes the menu.
            // Both layers are opaque: the pointer must not reach the editor
            // beneath (its hover over a word would close the menu), nor a
            // click in the gaps between the entries the area that closes it.
            let outside = column![
                Space::new().height(bounds.y + bounds.height),
                iced::widget::opaque(
                    iced::widget::mouse_area(Space::new().width(Length::Fill).height(Length::Fill))
                        .on_press(Message::CloseMenu)
                        .on_right_press(Message::CloseMenu)
                ),
            ];
            (
                outside.into(),
                iced::widget::opaque(dropdown),
                iced::Point::new(bounds.x, bounds.y + bounds.height),
            )
        } else {
            (
                Space::new().into(),
                Space::new().into(),
                iced::Point::ORIGIN,
            )
        };
        let with_dropdown: Element<Message> = crate::floating::at_point(
            iced::widget::stack![base, menu_outside],
            menu_dropdown,
            menu_at.x,
            menu_at.y,
        );

        // The card of a problem floats under it. The layer is always there,
        // empty when no card is open: the tree of widgets must keep its shape,
        // or the editor loses its focus (and its caret) each time a card
        // appears or goes away.
        let (card_at, card): (iced::Point, Element<Message>) = match self.lint_card(dark) {
            Some(open) => open,
            None => (iced::Point::ORIGIN, Space::new().into()),
        };
        let with_dropdown: Element<Message> =
            crate::floating::at_point(with_dropdown, card, card_at.x, card_at.y);
        // What the language server says of the name under the pointer.
        let (said_at, said): (iced::Point, Element<Message>) = match self.symbol_card(dark) {
            Some(open) => open,
            None => (iced::Point::ORIGIN, Space::new().into()),
        };
        let with_dropdown: Element<Message> =
            crate::floating::at_point(with_dropdown, said, said_at.x, said_at.y);
        // The list of completions, under the word being typed; its layer
        // is always there too, for the same reason.
        let (list_at, list): (iced::Point, Element<Message>) = match self.completion_view(dark) {
            Some(open) => open,
            None => (iced::Point::ORIGIN, Space::new().into()),
        };
        let with_dropdown: Element<Message> =
            crate::floating::at_point(with_dropdown, list, list_at.x, list_at.y);
        // The help on the parameters of the call being typed.
        let (hint_at, hint): (iced::Point, Element<Message>) = match self.hint_view(dark) {
            Some(open) => open,
            None => (iced::Point::ORIGIN, Space::new().into()),
        };
        let with_dropdown: Element<Message> =
            crate::floating::at_point(with_dropdown, hint, hint_at.x, hint_at.y);

        // The list of every tab, under its button; a click elsewhere closes it.
        let (list_outside, tabs_list, tabs_at): (Element<Message>, Element<Message>, iced::Point) =
            match self.tab_list {
                Some(at) => {
                    let outside = iced::widget::mouse_area(
                        Space::new().width(Length::Fill).height(Length::Fill),
                    )
                    .on_press(Message::CloseTabList)
                    .on_right_press(Message::CloseTabList);
                    (
                        outside.into(),
                        self.tab_list_view(dark),
                        iced::Point::new(
                            (at.x - views::TAB_LIST_WIDTH + 16.0).max(4.0),
                            at.y + 14.0,
                        ),
                    )
                }
                None => (
                    Space::new().into(),
                    Space::new().into(),
                    iced::Point::ORIGIN,
                ),
            };
        let with_dropdown: Element<Message> = crate::floating::at_point(
            iced::widget::stack![with_dropdown, list_outside],
            tabs_list,
            tabs_at.x,
            tabs_at.y,
        );

        // The Preferences window floats over everything but the dialogs.
        let with_prefs: Element<Message> = if self.show_preferences {
            let (x, y, w, h) = self.prefs_rect();
            let window = container(se_theme::in_popin(|| {
                self.preferences.view(
                    &self.config,
                    self.capturing,
                    &self.servers,
                    &self.plugins,
                    &self.spell.choices,
                )
            }))
            .width(Length::Fixed(w))
            .height(Length::Fixed(h));
            // The edges and the corner take hold of it to resize it.
            let grip = |grab, interaction, width, height| {
                iced::widget::mouse_area(Space::new().width(width).height(height))
                    .on_press(Message::PrefsDragStart(grab))
                    .interaction(interaction)
            };
            use crate::preferences::PrefsGrab;
            use iced::mouse::Interaction;
            let right = container(grip(
                PrefsGrab::Right,
                Interaction::ResizingHorizontally,
                Length::Fixed(PREFS_EDGE),
                Length::Fill,
            ))
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(iced::alignment::Horizontal::Right);
            let bottom = container(grip(
                PrefsGrab::Bottom,
                Interaction::ResizingVertically,
                Length::Fill,
                Length::Fixed(PREFS_EDGE),
            ))
            .width(Length::Fill)
            .height(Length::Fill)
            .align_y(iced::alignment::Vertical::Bottom);
            let corner = container(grip(
                PrefsGrab::Corner,
                Interaction::ResizingDiagonallyDown,
                Length::Fixed(PREFS_CORNER),
                Length::Fixed(PREFS_CORNER),
            ))
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(iced::alignment::Horizontal::Right)
            .align_y(iced::alignment::Vertical::Bottom);
            crate::floating::modal_at(
                with_dropdown,
                iced::widget::stack![window, right, bottom, corner],
                (x, y),
                Message::TogglePreferences,
                dark,
            )
        } else {
            with_dropdown
        };

        // The dialogs of the plugins, under the overlays of the editor.
        let with_prefs = self.plugin_dialogs_view(with_prefs, dark);

        // Modal overlays: popins, whose controls have rounded corners.
        se_theme::in_popin(|| match &self.overlay {
            ActiveOverlay::None => with_prefs,
            ActiveOverlay::Everywhere => {
                let card = self.view_everywhere(dark);
                crate::floating::modal(
                    with_prefs,
                    column![Space::new().height(70), card].align_x(Alignment::Center),
                    Some(Message::CloseOverlay),
                    dark,
                    true,
                )
            }
            ActiveOverlay::FindInFiles => {
                let card = self.view_find_in_files(dark);
                // Pinned, it stays when the editor is clicked.
                let blur = (!self.finder.pinned).then_some(Message::CloseOverlay);
                crate::floating::modal(
                    with_prefs,
                    column![Space::new().height(40), card].align_x(Alignment::Center),
                    blur,
                    dark,
                    true,
                )
            }
            ActiveOverlay::References(list) => {
                let card = self.view_references_overlay(dark, list);
                crate::floating::modal(
                    with_prefs,
                    column![Space::new().height(84), card].align_x(Alignment::Center),
                    Some(Message::CloseOverlay),
                    dark,
                    true,
                )
            }
            other => {
                let overlay = match other {
                    ActiveOverlay::About => self.view_about_overlay(dark),
                    ActiveOverlay::Welcome => self.view_welcome_overlay(dark),
                    ActiveOverlay::Changelog => {
                        self.changelog_card(dark, t!("changelog.title").to_string(), None)
                    }
                    ActiveOverlay::WhatsNew(previous) => self.changelog_card(
                        dark,
                        t!("changelog.whats_new", version = env!("CARGO_PKG_VERSION")).to_string(),
                        Some(previous.as_str()),
                    ),
                    ActiveOverlay::GotoLine => self.view_goto_line_overlay(dark),
                    ActiveOverlay::LanguagePicker => self.view_language_picker_overlay(dark),
                    ActiveOverlay::TextOptions(which) => self.view_text_options(dark, *which),
                    ActiveOverlay::Rename(name) => self.view_rename(dark, name),
                    ActiveOverlay::Compare(left, right) => self.view_compare(dark, left, right),
                    ActiveOverlay::Spelling(found) => self.view_spelling(dark, found),
                    ActiveOverlay::SpellLanguages => self.view_spell_languages(dark),
                    ActiveOverlay::ConfirmOverwrite(path) => {
                        self.view_overwrite_overlay(dark, path)
                    }
                    ActiveOverlay::ConfirmClose(path) => self.view_confirm_overlay(dark, path),
                    ActiveOverlay::PluginConfirm(q) => self.view_plugin_question(dark, q),
                    ActiveOverlay::ConfirmPluginUninstall(id, with_data) => {
                        let name = self
                            .plugins
                            .get(id)
                            .map_or_else(|| id.clone(), |p| p.name());
                        let with_data = *with_data;
                        confirm_card_kind(
                            dark,
                            t!("plugins.uninstall_title", name = name.as_str()).to_string(),
                            t!("plugins.uninstall_message").to_string(),
                            Some(
                                iced::widget::checkbox(with_data)
                                    .label(t!("plugins.uninstall_data").to_string())
                                    .size(15)
                                    .text_size(13)
                                    .on_toggle(|on| {
                                        Message::Plugin(
                                            crate::plugins::PluginMsg::UninstallDataToggled(on),
                                        )
                                    })
                                    .into(),
                            ),
                            t!("plugins.uninstall").to_string(),
                            Message::Plugin(if with_data {
                                crate::plugins::PluginMsg::UninstallWithData(id.clone())
                            } else {
                                crate::plugins::PluginMsg::Uninstall(id.clone())
                            }),
                            Confirm::Delete,
                        )
                    }
                    ActiveOverlay::ConfirmFileDelete(path, options) => confirm_card_kind(
                        dark,
                        t!("delete_file.title").to_string(),
                        t!("delete_file.message", name = file_label(path).as_str()).to_string(),
                        // The boxes of the plugins ("Also remove it from Git").
                        (!options.is_empty()).then(|| {
                            column(options.iter().enumerate().map(|(i, (_, option))| {
                                iced::widget::checkbox(option.checked)
                                    .label(option.label.clone())
                                    .size(15)
                                    .text_size(13)
                                    .on_toggle(move |on| {
                                        Message::Plugin(
                                            crate::plugins::PluginMsg::DeleteOptionToggled(i, on),
                                        )
                                    })
                                    .into()
                            }))
                            .spacing(6)
                            .into()
                        }),
                        t!("delete_file.confirm").to_string(),
                        Message::DeleteFile(path.clone()),
                        Confirm::Delete,
                    ),
                    ActiveOverlay::ProjectName(dialog) => self.view_name_overlay(dark, dialog),
                    ActiveOverlay::ConfirmMove(source, dir) => confirm_card_kind(
                        dark,
                        t!("projects.move_title").to_string(),
                        move_message(source, dir),
                        None,
                        t!("projects.move").to_string(),
                        Message::MoveEntry(source.clone(), dir.clone()),
                        Confirm::Primary,
                    ),
                    ActiveOverlay::ConfirmFolderDelete(dir) => confirm_delete_card(
                        dark,
                        t!("projects.delete_title").to_string(),
                        t!("projects.delete_message", name = file_label(dir).as_str()).to_string(),
                        t!("projects.delete_confirm").to_string(),
                        Message::DeleteFolder(dir.clone()),
                    ),
                    ActiveOverlay::ConfirmScratchDelete(path) => confirm_delete_card(
                        dark,
                        t!("scratch.delete_title").to_string(),
                        t!("scratch.delete_message", name = file_label(path).as_str()).to_string(),
                        t!("scratch.delete").to_string(),
                        Message::ScratchDelete(path.clone()),
                    ),
                    ActiveOverlay::ConfirmCloseMany(paths, modified) => {
                        self.view_confirm_many_overlay(dark, paths, *modified)
                    }
                    ActiveOverlay::ConfirmTrust(root) => {
                        let folder = root.display().to_string();
                        confirm_card_kind(
                            dark,
                            t!("trust.title").to_string(),
                            t!("trust.message", folder = folder.as_str()).to_string(),
                            None,
                            t!("trust.confirm").to_string(),
                            Message::TrustFolder(root.clone()),
                            Confirm::Primary,
                        )
                    }
                    ActiveOverlay::ConfirmQuit(error) => confirm_card(
                        dark,
                        t!("quit.title").to_string(),
                        if error == t!("quit.unsaved").as_ref() {
                            t!("quit.message_unsaved").to_string()
                        } else {
                            t!("quit.message", error = error.as_str()).to_string()
                        },
                        t!("quit.confirm").to_string(),
                        Message::QuitAnyway,
                    ),
                    _ => self.view_update_overlay(dark),
                };
                // A running download or install can't be dismissed: its
                // outcome would arrive without context. Escape reaches the
                // app through its own subscription, for every overlay.
                let on_blur = self
                    .overlay_is_dismissable()
                    .then_some(Message::CloseOverlay);
                crate::floating::modal(with_prefs, overlay, on_blur, dark, false)
            }
        })
    }

    pub fn theme(&self) -> Theme {
        se_theme::set_high_contrast(self.config.high_contrast);
        se_theme::toolkit_theme(self.config.dark_mode)
    }
}

// ─── Message handling ───────────────────────────────────────────────────────

impl Message {
    /// Answers of background work and timers: not an action of the user.
    fn is_background(&self) -> bool {
        match self {
            Message::CompareDiff(DiffNav::Scrolled(..)) => true,
            Message::Plugin(msg) => matches!(
                msg,
                crate::plugins::PluginMsg::Incoming(_)
                    | crate::plugins::PluginMsg::RegistryLoaded(_)
                    | crate::plugins::PluginMsg::Installed(..)
                    | crate::plugins::PluginMsg::UpdatesChecked(_)
                    | crate::plugins::PluginMsg::Fetched(_)
            ),
            Message::SpellChecked(_)
            | Message::SpellLearned(_) => true,
            // What comes by itself (answers, timers) or from the pointer
            // merely passing over something: an open menu stays open.
            Message::LintDone(..)
            | Message::LintFixesLoaded(..)
            | Message::LintCloseCheck(_)
            | Message::LintHover(_)
            | Message::LintCardHover(_)
            | Message::FinderFiles(_)
            | Message::FindResults(..)
            | Message::FindDue(_)
            | Message::TreeScrolled(..)
            | Message::TreeHover(_)
            | Message::EverywhereFiles(..)
            | Message::Results(results::ResultsMsg::Rerun_(..))
            | Message::Results(results::ResultsMsg::Exported(_))
            | Message::EverywhereSymbolsDue(_)
            | Message::EverywhereSymbols(..)
            | Message::EverywhereTextDue(_)
            | Message::EverywhereText(..)
            | Message::CompletionInfo(..)
            | Message::CompletionAnswer(..)
            | Message::CompletionAnchor(..)
            | Message::HintAnswer(..)
            | Message::CompletionWheel(_)
            | Message::CompletionResolved(..)
            | Message::CompletionBusy(..)
            | Message::SymbolHover(_)
            | Message::SymbolHoverLoaded(..)
            // The poll for files sent by another instance ticks every
            // 300 ms; a file it brings opens through its own messages.
            | Message::IpcPoll
            // A build writing in the project, the auto-save, a file
            // rewritten by another program, the output of a running task.
            | Message::FsChanged
            | Message::FileSaved(..)
            | Message::FileReloaded(_)
            | Message::LargeRefreshed(..)
            | Message::BookmarkTexts(_)
            | Message::FinderPreviewRead(..)
            | Message::ProjectsReloaded(_) => true,
            _ => false,
        }
    }

    /// Shortcuts that act on the buffer and so shouldn't fire while a
    /// text field is being typed in.
    fn is_editor_shortcut(&self) -> bool {
        matches!(
            self,
            Message::Undo
                | Message::Redo
                | Message::SelectAll
                | Message::DuplicateLine
                | Message::MoveLineUp
                | Message::MoveLineDown
                | Message::ToggleComment
                | Message::DeleteLine
                | Message::InsertTab
                | Message::DedentLine
        )
    }
}

impl ArdoiseApp {
    fn handle(&mut self, message: Message) -> Task<Message> {
        // Passive/background messages must not close an open dropdown: X11
        // fires a spurious ModifiersChanged right after the very click that
        // opens the menu, AutoSave ticks every 5s regardless of the UI, and
        // MenuBounds is this same open answering its own position query.
        let closes_menu = !message.is_background()
            && !matches!(
                &message,
                Message::ToggleMenu(_)
                    | Message::HoverMenu(_)
                    | Message::SetSubmenu(_)
                    | Message::MenuBounds(..)
                    | Message::ModifiersChanged { .. }
                    | Message::AutoSave
                    | Message::SessionWritten(_)
                    | Message::PreviewTick
                    | Message::WindowResized(..)
                    | Message::WindowFocus(_)
            );
        if closes_menu || matches!(message, Message::ToggleMenu(_) | Message::HoverMenu(_)) {
            self.open_submenu = None;
        }
        if closes_menu {
            // Escape on an open menu closes only that menu.
            if matches!(message, Message::EscapePressed) && self.open_menu.take().is_some() {
                return Task::none();
            }
            self.open_menu = None;
        }

        match message {
            Message::EditorAction(action) => {
                // The editor has the keyboard again.
                self.plugins.grid_focus = None;
                self.plugins.panel_focus = None;
                self.plugins.editor_focus = None;
                self.plugins.completion = None;
                self.symbol_card = None;
                self.symbol_token += 1;
                self.merge_typing = false;
                self.tree_focus = false;
                self.results.focus = false;
                if action.is_edit() {
                    self.blame = None;
                }
                // Without a tab there is nothing to edit (the next one is
                // still being read).
                if (self.current_file.is_none() || self.large_view().is_some()) && action.is_edit()
                {
                    return Task::none();
                }
                // Lines added or removed by an edit move the bookmarks below.
                let probe = action.is_edit().then(|| self.edit_probe());
                let completion_action = action.clone();
                let command = self.editor_action(action);
                if let Some((count, top)) = probe {
                    self.shift_bookmarks(count, top);
                }
                let completion = self.completion_after_edit(&completion_action);
                Task::batch([command, completion])
            }
            Message::InsertTab => {
                if !self.editing_blocked() {
                    if self.selection_span().is_some() {
                        return self.handle(Message::IndentSelection);
                    }
                    // With a single-line selection this replaces it, as
                    // in most editors.
                    let text = self.tab_text();
                    self.push_undo();
                    self.undo_new_group = true;
                    self.edited(|app| {
                        app.editor.content.perform(text_editor::Action::Edit(
                            text_editor::Edit::Paste(std::sync::Arc::new(text)),
                        ));
                    });
                    self.is_dirty = true;
                }
                Task::none()
            }
            Message::SelectAll => {
                self.editor.content.perform(text_editor::Action::Move(
                    text_editor::Motion::DocumentStart,
                ));
                self.editor.content.perform(text_editor::Action::Select(
                    text_editor::Motion::DocumentEnd,
                ));
                Task::none()
            }
            Message::SetSubmenu(open) => {
                self.open_submenu = open;
                Task::none()
            }
            Message::ClearRecents => {
                self.config.clear_recent();
                self.config.save();
                Task::none()
            }
            Message::ToggleMenu(menu) => {
                self.open_menu = if self.open_menu == Some(menu) {
                    None
                } else {
                    Some(menu)
                };
                self.query_menu_bounds()
            }
            Message::HoverMenu(menu) => {
                if self.open_menu.is_some() && self.open_menu != Some(menu) {
                    self.open_menu = Some(menu);
                    self.query_menu_bounds()
                } else {
                    Task::none()
                }
            }
            Message::MenuBounds(menu, bounds) => {
                if let Some(bounds) = bounds {
                    self.menu_bounds.insert(menu, bounds);
                }
                Task::none()
            }
            Message::NewFile => {
                self.stash_current();
                self.untitled_counter += 1;
                let path = untitled_path(self.untitled_counter);
                let mut blank = CachedBuffer::blank();
                // Stored as the settings say (encoding, line ending).
                blank.format = self.config.new_file_format();
                self.load_buffer(path, blank);
                self.set_status(t!("status.new_file"), false);
                Task::none()
            }
            Message::OpenFile => Task::perform(open_file(), Message::FileOpened),
            Message::FileOpened(result) => {
                match result {
                    Ok(loaded) => self.open_loaded(loaded),
                    Err(e) if e == CANCELLED => {}
                    Err(e) => self.set_status(format!("{}: {}", t!("status.error"), e), true),
                }
                Task::none()
            }
            Message::TabRead(result) => {
                match result {
                    // A slower read of a tab the user has left behind.
                    // (Its cached state stays for the next visit.)
                    Ok(loaded)
                        if self
                            .pending_open
                            .as_ref()
                            .is_some_and(|p| *p != loaded.path) => {}
                    // The user went elsewhere in the meantime.
                    Ok(loaded) if self.abandoned_reads.remove(&loaded.path) => {}
                    Ok(loaded) => {
                        self.open_loaded(loaded);
                        self.apply_pending_jump();
                    }
                    Err(e) => {
                        self.pending_jump = None;
                        self.set_status(format!("{}: {}", t!("status.error"), e), true);
                        let failed = self.pending_open.take();
                        // Without a tab to show, drop the one that can't be read.
                        if self.current_file.is_none() {
                            if let Some(path) = failed {
                                self.sidebar.remove_path(&path);
                                self.file_cache.remove(&path);
                            }
                            return self.ensure_tab();
                        }
                    }
                }
                Task::none()
            }
            Message::FileReloaded(result) => {
                match result {
                    // Too large to open whole: only its first part came.
                    Ok(loaded) if self.large.contains_key(&loaded.path) => {}
                    Ok(loaded) => {
                        // Changed on the disk: the plugins that follow it hear of it.
                        self.plugins_file_event(
                            ardoise_plugin::manifest::Event::FileSaved,
                            &loaded.path,
                        );
                        self.apply_reload(loaded);
                    }
                    Err(e) => self.set_status(format!("{}: {}", t!("status.error"), e), true),
                }
                Task::none()
            }
            Message::SaveFile if self.large_view().is_some() => {
                self.set_status(t!("large.read_only"), true);
                Task::none()
            }
            Message::SaveFile => {
                // A tab of a plugin is the plugin's to keep.
                if let Some(tab) = self
                    .current_file
                    .as_ref()
                    .and_then(|p| self.plugins.tabs.get(p))
                {
                    crate::plugins::host::notify(
                        &tab.plugin,
                        ardoise_plugin::protocol::methods::TAB_SAVE,
                        ardoise_plugin::protocol::PanelParams {
                            panel: tab.tab.clone(),
                        },
                    );
                    return Task::none();
                }
                // Format first, when asked to: the save follows the formatting.
                let skip_format = std::mem::take(&mut self.skip_format_once);
                if self.config.format_on_save
                    && !skip_format
                    && !self.is_formatting
                    && self.format_target().is_some()
                    && self
                        .editor
                        .language
                        .as_deref()
                        .is_some_and(crate::formatter::can_format)
                {
                    self.format_on_save_pending = true;
                    return self.handle(Message::FormatFile);
                }
                self.fix_before_save();
                self.trim_before_save();
                if let Some(path) = self.current_file.clone() {
                    if self.changed_on_disk(&path) {
                        self.overlay = ActiveOverlay::ConfirmOverwrite(path);
                        return Task::none();
                    }
                }
                let content = crate::editor::buffer_text(&self.editor.content);
                let origin = self.current_file.clone();
                match origin.clone() {
                    Some(path) if !is_untitled(&path) => save_command(
                        origin,
                        save_file(path, content, self.save_format(self.format)),
                    ),
                    Some(_) => self.start_save_as(origin),
                    None => Task::none(),
                }
            }
            Message::SaveAll => {
                let mut tasks = Vec::new();
                // The tab on screen goes through the usual save; a tab
                // without a file waits for the user to name it.
                let on_screen = self
                    .current_file
                    .as_ref()
                    .is_some_and(|p| !is_untitled(p) && !is_virtual(p));
                if self.is_dirty && on_screen {
                    tasks.push(self.handle(Message::SaveFile));
                }
                tasks.extend(self.save_cached_tabs());
                Task::batch(tasks)
            }
            Message::SaveFileAs => self.start_save_as(self.current_file.clone()),
            Message::SaveAsChosen(origin, result) => match result {
                // The buffer only holds part of a large file.
                Ok(_) if origin.as_ref().is_some_and(|o| self.large.contains_key(o)) => {
                    self.set_status(t!("large.read_only"), true);
                    Task::none()
                }
                Ok(path) => {
                    // Another tab holds unsaved work for that file: writing
                    // over the file would silently drop it.
                    let elsewhere = origin.as_ref() != Some(&path)
                        && self.sidebar.files().contains(&path)
                        && self.is_path_dirty(&path);
                    if elsewhere {
                        self.set_status(
                            t!("status.save_as_open_tab", name = file_label(&path).as_str()),
                            true,
                        );
                        return Task::none();
                    }
                    // Whatever the tab holds now is what gets written.
                    let content = if origin == self.current_file {
                        crate::editor::buffer_text(&self.editor.content)
                    } else {
                        let text = origin
                            .as_ref()
                            .and_then(|o| self.file_cache.get(o))
                            .and_then(CachedBuffer::text);
                        // The tab was closed while the dialog was open, or its
                        // text is no longer loaded: an empty text would wipe
                        // the chosen file.
                        let Some(text) = text else {
                            self.set_status(t!("status.save_as_tab_closed"), true);
                            return Task::none();
                        };
                        text
                    };
                    let format = if origin == self.current_file {
                        self.format
                    } else {
                        origin
                            .as_ref()
                            .and_then(|o| self.file_cache.get(o))
                            .map(|c| c.format)
                            .unwrap_or_default()
                    };
                    save_command(origin, save_file(path, content, self.save_format(format)))
                }
                Err(e) if e == CANCELLED => Task::none(),
                Err(e) => {
                    self.set_status(format!("{}: {}", t!("status.error"), e), true);
                    Task::none()
                }
            },
            Message::FileSaved(origin, result) => {
                match result {
                    Ok((path, saved)) => {
                        self.plugins_file_event(ardoise_plugin::manifest::Event::FileSaved, &path);
                        self.size_for = None;
                        // The tab the save started from may no longer be the
                        // active one (the dialog is asynchronous).
                        let from_current = origin == self.current_file;
                        if let Some(old) = origin.as_ref().filter(|o| **o != path) {
                            // "Save as" turns the tab into the new file; the
                            // old file stays as it is on disk.
                            if self.sidebar.files().contains(&path) {
                                self.sidebar.remove_path(&path);
                                self.undo_cache.remove(&path);
                            }
                            self.file_cache.remove(&path);
                            self.sidebar.rename_file(old, path.clone());
                            self.bookmarks.rename(old, &path);
                            if !from_current {
                                if let Some(entry) = self.file_cache.remove(old) {
                                    self.file_cache.insert(path.clone(), entry);
                                }
                                if let Some(stacks) = self.undo_cache.remove(old) {
                                    self.undo_cache.insert(path.clone(), stacks);
                                }
                            }
                        }
                        // Not for a tab closed while the save was running.
                        let still_open = origin.as_ref().is_none_or(|o| {
                            is_untitled(o)
                                || self.sidebar.files().contains(o)
                                || self.sidebar.files().contains(&path)
                        });
                        if still_open && !self.is_scratch(&path) {
                            self.sidebar.add_file(path.clone());
                        }
                        let saved_fp = fingerprint(&saved);
                        let stamp = DiskStamp::of(&path);
                        if from_current {
                            let first_line = first_line_of(&self.editor.content);
                            self.editor.adopt_path_language(&path, &first_line);
                            // Edits made while the save was in flight stay dirty.
                            let now = crate::editor::buffer_text(&self.editor.content);
                            self.is_dirty = now != saved;
                            self.saved_fp = Some(saved_fp);
                            self.disk_stamp = stamp;
                            self.external_warned = false;
                            self.current_file = Some(path.clone());
                        } else if let Some(entry) = self.file_cache.get_mut(&path) {
                            entry.dirty = entry.text().is_some_and(|t| t != saved);
                            entry.saved_fp = Some(saved_fp);
                            entry.stamp = stamp;
                        }
                        if !self.is_scratch(&path) {
                            self.config.add_recent(&path);
                            self.config_dirty = true;
                        }
                        if self.autosaving == 0 {
                            self.set_status(t!("status.file_saved"), false);
                        }
                        self.autosaving = self.autosaving.saturating_sub(1);
                        // The tools that read the disk look again.
                        self.lint.force = self.config.lint_trigger != crate::lint::Trigger::Manual;
                        return Task::future(crate::lsp::did_save(path.clone(), saved.clone()))
                            .discard();
                    }
                    Err(e) if e == CANCELLED => self.autosaving = self.autosaving.saturating_sub(1),
                    Err(e) => {
                        self.autosaving = self.autosaving.saturating_sub(1);
                        self.set_status(format!("{}: {}", t!("status.error"), e), true);
                    }
                }
                Task::none()
            }
            Message::CloseOtherTabs => self.handle(Message::CloseTabs(self.current_file.clone())),
            Message::CloseTabs(keep) => self.request_close_many(keep),
            Message::ConfirmDiscardMany(paths) => {
                self.overlay = ActiveOverlay::None;
                self.close_many(paths)
            }
            Message::CloseFile => match self.current_file.clone() {
                // Scratch files stay until they are deleted.
                Some(path) if self.is_scratch(&path) => Task::none(),
                Some(path) => self.request_close(path),
                None => Task::none(),
            },
            // The menu closed above, like for any message that isn't passive.
            Message::CloseMenu => Task::none(),
            Message::BlameFile => self.git_command("git.blame"),
            Message::CloseBlame => {
                self.blame = None;
                self.git_command("git.blame_close")
            }
            Message::BlameOption(option) => {
                self.plugins.pointed_line = self.gutter_line;
                self.git_command(match option {
                    BlameOption::Blanks => "git.blame_blanks",
                    BlameOption::MovesInFile => "git.blame_moves",
                    BlameOption::MovesAcross => "git.blame_across",
                    BlameOption::ShowHidden => "git.blame_show_hidden",
                    BlameOption::HideLine => "git.blame_hide",
                })
            }
            Message::RunMarkClicked(line) => {
                self.plugin_run_mark_clicked(line);
                Task::none()
            }
            Message::BlameLineClicked(line) => {
                self.plugin_annotation_clicked(line);
                Task::none()
            }
            Message::BlamePrevious => {
                self.plugins.pointed_line = self.gutter_line;
                self.git_command("git.blame_previous")
            }
            Message::Merge(msg) => self.merge_msg(msg),
            Message::MergeApplied(path, result) => match result {
                Ok(()) => self.close_path(path),
                Err(error) => {
                    self.set_status(format!("{}: {error}", t!("status.error")), true);
                    Task::none()
                }
            },
            Message::DiffTab(nav) => self.diff_nav(nav),
            // Git is the Git plugin's: its panel opens or closes.
            Message::ToggleGit => {
                // Without the Git plugin, what goes with it is still there.
                if !self.plugins.tools().git && !self.plugins.git_group().is_empty() {
                    return self.update(Message::Plugin(crate::plugins::PluginMsg::ToggleGitGroup));
                }
                let open = self
                    .plugins
                    .bottom
                    .as_ref()
                    .is_some_and(|k| k.0 == crate::plugins::migrate::GIT);
                if open {
                    self.plugin_bottom_yield();
                    return Task::none();
                }
                self.git_command("git.show")
            }
            // The terminal is the Terminal plugin's: its panel opens or closes.
            Message::ToggleTerminal => {
                let open = self
                    .plugins
                    .bottom
                    .as_ref()
                    .is_some_and(|k| k.0 == crate::plugins::migrate::TERMINAL);
                if open {
                    self.plugin_bottom_yield();
                    return Task::none();
                }
                self.run_official_command(
                    crate::plugins::migrate::TERMINAL,
                    "terminal.toggle",
                    t!("terminal.plugin_missing").to_string(),
                )
            }
            Message::ToggleScratch => {
                self.scratch.open = !self.scratch.open;
                if self.scratch.open {
                    self.plugin_bottom_yield();
                    self.bookmark_panel.open = false;
                    self.scratch.refresh();
                    self.scratch.error = None;
                }
                Task::none()
            }
            Message::ToggleBookmarkPanel => {
                self.bookmark_panel.open = !self.bookmark_panel.open;
                if self.bookmark_panel.open {
                    self.plugin_bottom_yield();
                    self.scratch.open = false;
                    return self.refresh_bookmark_texts(true);
                }
                Task::none()
            }
            Message::BookmarkClick(target) => self.bookmark_click(target),
            Message::BookmarkTexts(read) => {
                self.bookmark_texts_read(read);
                Task::none()
            }
            Message::BookmarkFold(path) => {
                if !self.bookmark_panel.collapsed.remove(&path) {
                    self.bookmark_panel.collapsed.insert(path);
                }
                Task::none()
            }
            Message::BookmarkDelete(target) => {
                self.delete_bookmark(target);
                Task::none()
            }
            Message::ToggleFileBookmark(path) => {
                self.toggle_file_bookmark(path);
                Task::none()
            }
            Message::ScratchName(name) => {
                self.scratch.name = name;
                self.scratch.error = None;
                Task::none()
            }
            Message::ScratchExt(ext) => {
                self.scratch.ext = ext;
                self.scratch.error = None;
                Task::none()
            }
            Message::ScratchCreate => {
                let ext = self.scratch.ext.as_str();
                let name = if self.scratch.name.trim().is_empty() {
                    crate::scratch::next_name(&self.scratch.dir, ext)
                } else {
                    self.scratch.name.clone()
                };
                match crate::scratch::create(&self.scratch.dir, &name, ext) {
                    Ok(path) => {
                        self.scratch.name.clear();
                        self.scratch.error = None;
                        self.scratch.refresh();
                        self.open_scratch(path)
                    }
                    Err(e) => {
                        self.scratch.error = Some(e);
                        Task::none()
                    }
                }
            }
            Message::ScratchOpen(path) => self.open_scratch(path),
            Message::ScratchAskDelete(path) => {
                self.overlay = ActiveOverlay::ConfirmScratchDelete(path);
                Task::none()
            }
            Message::ScratchDelete(path) => {
                self.overlay = ActiveOverlay::None;
                let command = self.close_path(path.clone());
                self.bookmarks.clear(&path);
                if let Err(e) = crate::fsutil::trash_or_remove(&path) {
                    if e.kind() != std::io::ErrorKind::NotFound {
                        self.scratch.error = Some(e.to_string());
                    }
                }
                self.scratch.refresh();
                command
            }
            Message::PickScratchDir => Task::perform(
                async {
                    rfd::AsyncFileDialog::new()
                        .set_title(t!("scratch.folder_title").to_string())
                        .pick_folder()
                        .await
                        .map(|h| h.path().to_path_buf())
                },
                Message::ScratchDirChosen,
            ),
            Message::ScratchDirChosen(Some(dir)) => {
                self.set_scratch_dir(dir.to_string_lossy().into_owned())
            }
            Message::ScratchDirChosen(None) => Task::none(),
            Message::SetScratchDir(dir) => self.set_scratch_dir(dir),
            Message::ConfirmOverwrite => {
                self.overlay = ActiveOverlay::None;
                // Take the new on-disk state as the baseline, then save.
                self.disk_stamp = self.current_file.as_deref().and_then(DiskStamp::of);
                self.handle(Message::SaveFile)
            }
            Message::ConfirmDiscard(path) => {
                self.overlay = ActiveOverlay::None;
                self.close_path(path)
            }
            Message::ToggleSearch => {
                self.show_search = !self.show_search;
                if self.show_search {
                    // Ready to type, as the card opens.
                    iced::widget::operation::focus(crate::search::find_field_id())
                } else {
                    Task::none()
                }
            }
            Message::Lint(setting) => {
                crate::lint::apply_setting(&mut self.config, setting);
                self.config_dirty = true;
                Task::none()
            }
            Message::LintDone(generation, key, outcome) => {
                self.lint_done(generation, key, outcome);
                Task::none()
            }
            Message::LintHover(over) => self.lint_hover(over),
            Message::CompletionInvoke => {
                // In an editor of a panel: the plugin is asked.
                if let Some(slot) = self.plugins.editor_focus.clone() {
                    return self.plugin_complete(slot);
                }
                self.completion_request(crate::lsp::Trigger::Invoked, true)
            }
            Message::CompletionInfo(key, info) => {
                self.completion_info_loaded(key, info);
                Task::none()
            }
            Message::CompletionAnswer(generation, answer) => {
                let (base, found) = *answer;
                self.completion_answer(generation, base, found)
            }
            Message::CompletionAnchor(list, hint) => {
                if let Some(session) = self.completion.session.as_mut() {
                    session.anchor_at = list;
                }
                if let Some(open) = self.completion.hint.as_mut() {
                    open.anchor_at = hint;
                }
                Task::none()
            }
            Message::HintInvoke => self.hint_request(1, None, true),
            Message::HintAnswer(generation, answer) => self.hint_answer(generation, answer),
            Message::CompletionPick(pos) => self.completion_accept(pos, false),
            Message::CompletionWheel(lines) => {
                self.completion_wheel(lines);
                Task::none()
            }
            Message::CompletionResolved(of, index, resolved) => {
                self.completion_resolved(of, index, resolved);
                Task::none()
            }
            Message::CompletionImport(token, resolved) => self.completion_import(token, resolved),
            Message::CompletionBusy(generation, busy) => {
                self.completion_busy_answer(generation, busy)
            }
            Message::LintCardHover(over) => self.lint_card_hovered(over),
            Message::LintCloseCheck(token) => {
                self.lint_close_check(token);
                Task::none()
            }
            Message::LintFixesLoaded(key, id, result) => {
                self.lint_fixes_loaded(key, id, result);
                Task::none()
            }
            Message::LintApply(id, fix) => self.lint_apply(id, fix),
            Message::LintFixAll => {
                self.lint.hover = None;
                self.start_fix_all()
            }
            Message::LintFixSource(source) => self.start_fix_source(source),
            Message::LintFixedAll(key, text) => self.lint_fixed_all(key, text),
            Message::LintFixFile(path) => self.lint_fix_file(path),
            Message::LintNow => self.lint_now(),
            Message::AddProject => Task::perform(pick_folder(), Message::ProjectPicked),
            Message::ProjectPicked(Some(dir)) => {
                if self.projects.add(dir) {
                    self.save_projects();
                }
                Task::none()
            }
            Message::ProjectPicked(None) => Task::none(),
            Message::ToggleProjectFolder(dir) => {
                self.tree_focus = true;
                self.projects.selected = Some(dir.clone());
                self.projects.toggle(&dir);
                self.save_projects();
                Task::none()
            }
            Message::RemoveProject(root) => {
                self.projects.remove(&root);
                self.save_projects();
                Task::none()
            }
            Message::NewProjectEntry(dir, folder) => self.ask_name(if folder {
                project_ops::NameFor::Folder(dir)
            } else {
                project_ops::NameFor::File(dir)
            }),
            Message::RenameProjectEntry(path) => self.ask_name(project_ops::NameFor::Rename(path)),
            Message::ProjectNameChanged(text) => {
                self.name_typed(text);
                Task::none()
            }
            Message::ProjectNameSubmit => self.name_submitted(),
            Message::AskDeleteFolder(dir) => {
                self.ask_delete_folder(dir);
                Task::none()
            }
            Message::DeleteFolder(dir) => self.delete_folder(dir),
            Message::FolderDeleted(result) => self.folder_deleted(result),
            Message::CopyPath(path) => iced::clipboard::write(path.to_string_lossy().into_owned()),
            Message::ToggleHideIgnored => {
                let hide = !self.projects.hide_ignored;
                self.projects.set_hide_ignored(hide);
                self.config.projects_hide_ignored = hide;
                self.config_dirty = true;
                Task::none()
            }
            Message::TreePress(path) => {
                self.tree_focus = true;
                self.projects.selected = Some(path.clone());
                self.tree_drag = Some((path, false));
                Task::none()
            }
            Message::TreeHover(over) => {
                if let Some((source, moved)) = &mut self.tree_drag {
                    if *source != over {
                        *moved = true;
                    }
                    if *moved {
                        // Onto a file: into its folder.
                        let dir = if over.is_dir() {
                            Some(over)
                        } else {
                            over.parent().map(Path::to_path_buf)
                        };
                        let source = source.clone();
                        self.projects.drop_target = dir
                            .filter(|d| movable(&source, d))
                            .filter(|d| self.projects.roots.iter().any(|r| d.starts_with(r)));
                    }
                }
                Task::none()
            }
            Message::TreeRelease => {
                let drag = self.tree_drag.take();
                let target = self.projects.drop_target.take();
                if let (Some((source, true)), Some(dir)) = (drag, target) {
                    self.overlay = ActiveOverlay::ConfirmMove(source, dir);
                }
                Task::none()
            }
            Message::MoveEntry(source, dir) => {
                self.overlay = ActiveOverlay::None;
                let Some(name) = source.file_name() else {
                    return Task::none();
                };
                let target = dir.join(name);
                if target.exists() {
                    let name = name.to_string_lossy().into_owned();
                    self.set_status(t!("projects.exists", name = name.as_str()), true);
                    return Task::none();
                }
                self.projects.open(&dir);
                self.move_entry(source, target)
            }
            Message::TreeScrolled(offset, height) => {
                self.tree_view = (offset, height);
                Task::none()
            }
            Message::CollapseProjects => {
                self.projects.collapse_all();
                self.save_projects();
                Task::none()
            }
            Message::SetProjectsExclude(typed) => {
                let patterns: Vec<String> = typed
                    .split(',')
                    .map(|p| p.trim().to_string())
                    .filter(|p| !p.is_empty())
                    .collect();
                self.preferences.exclude_draft = patterns.join(", ");
                self.projects.set_exclude(patterns.clone());
                self.config.projects_exclude = patterns;
                self.config_dirty = true;
                Task::none()
            }
            Message::OpenTerminalAt(dir) => {
                if self
                    .plugins
                    .get(crate::plugins::migrate::TERMINAL)
                    .is_none()
                {
                    self.set_status(t!("terminal.plugin_missing"), true);
                    return Task::none();
                }
                self.plugin_update(crate::plugins::PluginMsg::RunAt(
                    crate::plugins::migrate::TERMINAL.to_string(),
                    "terminal.here".to_string(),
                    dir,
                ))
            }
            Message::RevealInFileManager(path) => {
                if let Err(e) = reveal_in_file_manager(&path) {
                    self.set_status(format!("{}: {e}", t!("status.error")), true);
                }
                Task::none()
            }
            Message::EntryDuplicated(target, result) => self.entry_duplicated(target, result),
            Message::DuplicateEntry(path) => self.ask_name(project_ops::NameFor::Duplicate(path)),
            Message::ShowInProjects(file) => {
                if !self.projects.show(&file) {
                    return Task::none();
                }
                self.save_projects();
                // The tree is brought up: the sidebar, with the projects.
                self.panels = crate::sidebar::Panels {
                    files: false,
                    projects: true,
                };
                self.show_sidebar = true;
                self.config.show_sidebar = true;
                self.config.show_files_panel = false;
                self.config.show_projects_panel = true;
                self.config_dirty = true;
                // And scrolled to the file.
                let index = self
                    .projects
                    .rows()
                    .iter()
                    .position(|r| r.entry.path == file)
                    .unwrap_or(0);
                iced::widget::operation::scroll_to(
                    crate::sidebar::tree_id(),
                    iced::widget::scrollable::AbsoluteOffset {
                        x: 0.0,
                        y: crate::sidebar::tree_offset(index),
                    },
                )
            }
            Message::RefreshProjects => {
                self.projects.refresh(true);
                Task::none()
            }
            Message::OpenLintSettings => {
                self.show_preferences = true;
                // Without the Lint plugin there is no Lint page: the plugins.
                let page = if self.plugins.linter().is_some() {
                    crate::preferences::PrefsPage::Lint
                } else {
                    crate::preferences::PrefsPage::Plugins
                };
                self.handle(Message::Preferences(PreferencesMessage::PageSelected(page)))
            }
            Message::LintNext(forward) => {
                self.lint_next(forward);
                Task::none()
            }
            Message::Split(side_by_side) => self.split_editor(side_by_side, None),
            Message::SplitWith(path, side_by_side) => self.split_editor(side_by_side, Some(path)),
            Message::CloseSplit => match self.split.as_ref().map(|s| s.active) {
                Some(active) => self.close_pane(active),
                None => Task::none(),
            },
            Message::ClosePane(id) => self.close_pane(id),
            Message::SplitFocus(id) => self.focus_pane(id),
            Message::SplitAction(id, action) => self.split_action(id, action),
            Message::TabMoved(from, to) => {
                self.tab_moved(from, to);
                Task::none()
            }
            Message::TabGroupToggle(project) => {
                if !self.folded_tabs.remove(&project) {
                    self.folded_tabs.insert(project);
                }
                Task::none()
            }
            Message::CompareDiff(nav) => self.compare_diff_nav(nav),
            Message::TabGroupClose(project) => {
                let targets = self.tabs_of(&project);
                self.folded_tabs.remove(&project);
                self.confirm_close_many(targets, Task::none())
            }
            Message::ProjectColor(root, color) => {
                let key = root.to_string_lossy().into_owned();
                self.config.project_colors.insert(key, color);
                self.config_dirty = true;
                Task::none()
            }
            Message::TreeOpenFile(path) => self.tree_open(path),
            Message::KeepTab => {
                self.preview_tab = None;
                Task::none()
            }
            Message::TabDoubleClick(index) => {
                if self.preview_tab.is_some() && self.preview_tab == self.tab_at(index) {
                    self.preview_tab = None;
                }
                Task::none()
            }
            Message::CloseTabAt(index) => match self.tab_at(index) {
                Some(path) => self.request_close(path),
                None => Task::none(),
            },
            Message::ReopenTab => self.reopen_tab(),
            Message::OpenGotoFile => self.open_everywhere(crate::everywhere::Tab::Files),
            Message::OpenFindInFiles => self.open_find_in_files(),
            Message::FinderFiles(files) => self.finder_files(files),
            Message::FindQuery(text) => {
                self.finder.text = text;
                self.search_files(true)
            }
            Message::FindToggleCase => {
                self.finder.case_sensitive = !self.finder.case_sensitive;
                self.search_files(false)
            }
            Message::FindToggleRegex => {
                self.finder.regex = !self.finder.regex;
                self.search_files(false)
            }
            Message::FindDue(generation) => self.find_due(generation),
            Message::FindResults(generation, result) => {
                self.find_results(generation, result);
                Task::none()
            }
            Message::FindOpen(pick) => self.find_open(pick),
            Message::FindInFolder(dir) => self.find_in_folder(dir, false),
            Message::ReplaceInFolder(dir) => self.find_in_folder(dir, true),
            Message::OpenReplaceInFiles => self.open_finder(true),
            Message::FindReplaceText(text) => {
                self.finder.replace = text;
                self.search_files(true)
            }
            Message::FindToggleMultiline => self.finder_toggle_multiline(),
            Message::FindIgnored(on) => {
                self.config.find_ignored = on;
                self.config_dirty = true;
                self.load_scope_files()
            }
            Message::FindLines(action) => self.finder_lines_action(action),
            Message::FindToggleWords => {
                self.finder.words = !self.finder.words;
                self.search_files(false)
            }
            Message::FindTogglePreserve => {
                self.finder.preserve_case = !self.finder.preserve_case;
                self.search_files(false)
            }
            Message::FindMode(replace) => {
                self.finder.replace_mode = replace;
                self.search_files(false)
            }
            Message::FindContext(context) => {
                self.finder.context = context;
                self.finder.menu = None;
                self.search_files(false)
            }
            Message::FindMaskOn(on) => {
                self.finder.mask_on = on;
                self.search_files(false)
            }
            Message::FindMask(mask) => {
                self.finder.mask = mask;
                self.search_files(true)
            }
            Message::FindScope(scope) => {
                let reload = (scope == finder::Scope::Directory)
                    != (self.finder.scope == finder::Scope::Directory);
                self.finder.scope = scope;
                if reload {
                    self.load_scope_files()
                } else {
                    self.search_files(false)
                }
            }
            Message::FindPickDir => Task::perform(
                async {
                    rfd::AsyncFileDialog::new()
                        .set_title(t!("findrep.pick_dir").to_string())
                        .pick_folder()
                        .await
                        .map(|h| h.path().to_path_buf())
                },
                Message::FindDirPicked,
            ),
            Message::FindDirPicked(dir) => {
                let Some(dir) = dir else {
                    return Task::none();
                };
                self.set_finder_dir(dir);
                self.finder.scope = finder::Scope::Directory;
                self.load_scope_files()
            }
            Message::FindDirText(text) => self.finder_dir_typed(text),
            // Files over SSH are the Remote Files plugin's.
            Message::OpenRemote => self.run_official_command(
                crate::plugins::migrate::REMOTE,
                "remote.open",
                t!("remote.plugin_missing").to_string(),
            ),
            Message::SpellFix => self.open_spell_fix(),
            Message::SpellSuggested(found) => {
                self.spell_suggested(found);
                Task::none()
            }
            Message::SpellLearned(error) => self.spell_learned(error),
            Message::SpellChoices(codes) => {
                self.spell.choices = codes;
                Task::none()
            }
            Message::SpellLanguagesLoaded(codes) => {
                self.spell_languages_loaded(codes);
                Task::none()
            }
            Message::SpellChecked(done) => {
                self.spell_checked(done);
                Task::none()
            }
            Message::SpellReplace(word) => self.spell_replace(word),
            Message::SpellAdd => self.spell_learn(true),
            Message::SpellIgnore => self.spell_learn(false),
            Message::OpenSpellLanguages => self.open_spell_languages(),
            Message::SpellLanguage(code) => self.spell_set_language(code),
            Message::AskCompareClipboard => iced::clipboard::read().map(Message::CompareClipboard),
            Message::CompareClipboard(text) => self.compare_with_clipboard(text),
            Message::CompareDisk => self.compare_with_disk(),
            Message::ComparePickFile => self.compare_pick_file(),
            Message::Reloaded(path, revert, result) => self.reloaded(path, revert, result),
            Message::CompareDiskRead(result) => self.compare_disk_read(result),
            Message::CompareFilePicked(result) => self.compare_with_file(result),
            Message::OpenTodos => self.find_todos(),
            Message::TodosFound(result) => {
                match result {
                    Ok(hits) if hits.is_empty() => self.set_status(t!("todo.none"), false),
                    Ok(hits) => {
                        let n = hits.len();
                        let title = t!("todo.title", n = n).to_string();
                        self.results_add(title, None, hits);
                        self.set_status(t!("todo.found", n = n), false);
                    }
                    Err(e) => self.set_status(format!("{}: {e}", t!("status.error")), true),
                }
                Task::none()
            }
            // The shortcut of the tasks runs the Tasks plugin's command.
            Message::OpenTaskPicker => self.run_official_command(
                crate::plugins::migrate::TASKS,
                "tasks.run",
                t!("tasks.plugin_missing").to_string(),
            ),
            Message::IpcPoll => {
                let mut tasks = Vec::new();
                for batch in crate::ipc::take() {
                    for target in batch {
                        tasks.push(self.open_target(target));
                    }
                    // The window comes forward for the file it was given.
                    tasks.push(iced::window::latest().and_then(iced::window::gain_focus));
                }
                Task::batch(tasks)
            }
            Message::TargetOpened(target, result) => {
                let opened = result.as_ref().ok().map(|l| l.path.clone());
                let task = self.handle(Message::FileOpened(result));
                if let (Some(path), Some(line)) = (opened, target.line) {
                    if self.current_file.as_ref() == Some(&path) {
                        let col = target.col.unwrap_or(1).saturating_sub(1);
                        let row = line
                            .saturating_sub(1)
                            .min(self.editor.content.line_count().saturating_sub(1));
                        let col = self
                            .editor
                            .content
                            .line(row)
                            .map_or(0, |l| byte_of_char(&l, col));
                        self.editor.content.select_bytes(row, col, 0);
                    }
                }
                task
            }
            Message::TrustFolder(root) => {
                crate::trust::add(&root);
                let list = &mut self.config.trusted_folders;
                let text = root.to_string_lossy().into_owned();
                if !list.contains(&text) {
                    list.push(text);
                }
                self.config_dirty = true;
                self.overlay = ActiveOverlay::None;
                // The servers start again, now with their build scripts.
                Task::batch([
                    self.lint_now(),
                    Task::future(crate::lsp::restart_servers()).discard(),
                ])
            }
            Message::NewWindow => {
                // The new window reads the settings from the disk.
                if self.config_dirty && self.session_owner {
                    self.config_dirty = false;
                    self.config.save();
                }
                let started = std::env::current_exe()
                    .and_then(|exe| std::process::Command::new(exe).arg("--new-window").spawn());
                match started {
                    // The window that closes is waited for, or it stays a
                    // zombie until this one ends.
                    Ok(mut child) => {
                        std::thread::spawn(move || {
                            let _ = child.wait();
                        });
                    }
                    Err(e) => self.set_status(format!("{}: {e}", t!("status.error")), true),
                }
                Task::none()
            }
            Message::OpenOutline => self.open_outline(),
            Message::OutlineLoaded(path, result) => self.outline_loaded(path, result),
            Message::RenameSymbol => {
                if self
                    .editor
                    .language
                    .as_deref()
                    .is_none_or(|e| crate::lsp::server_of(e).is_none())
                {
                    self.set_status(t!("rename.no_server"), true);
                    return Task::none();
                }
                let word = self.word_at_cursor();
                if word.is_empty() {
                    self.set_status(t!("rename.no_name"), true);
                    return Task::none();
                }
                self.overlay = ActiveOverlay::Rename(word);
                Task::batch([
                    iced::widget::operation::focus(rename_field_id()),
                    iced::widget::operation::select_all(rename_field_id()),
                ])
            }
            Message::RenameInput(name) => {
                if let ActiveOverlay::Rename(current) = &mut self.overlay {
                    *current = name;
                }
                Task::none()
            }
            Message::RenameConfirm => {
                let ActiveOverlay::Rename(name) =
                    std::mem::replace(&mut self.overlay, ActiveOverlay::None)
                else {
                    return Task::none();
                };
                let name = name.trim().to_string();
                let (Some(path), Some(ext)) =
                    (self.current_file.clone(), self.editor.language.clone())
                else {
                    return Task::none();
                };
                if name.is_empty() || name == self.word_at_cursor() {
                    return Task::none();
                }
                let text = crate::editor::buffer_text(&self.editor.content);
                let (line, col) = self.editor.content.cursor_position();
                let source = fingerprint(&text);
                self.set_status(t!("rename.title"), false);
                Task::perform(
                    async move { crate::lsp::rename(&ext, &path, &text, line, col, &name).await },
                    move |result| Message::Renamed(source, result),
                )
            }
            Message::Renamed(source, result) => self.renamed(source, result),
            Message::SymbolHover(over) => {
                self.symbol_card = None;
                self.symbol_token += 1;
                let (Some((line, col, at)), Some(path), Some(ext)) = (
                    over,
                    self.current_file.clone().filter(|p| !is_untitled(p)),
                    self.editor.language.clone(),
                ) else {
                    return Task::none();
                };
                if self.lint.hover.is_some()
                    || self.large_view().is_some()
                    || crate::lsp::server_of(&ext).is_none()
                {
                    return Task::none();
                }
                // The text is made once per state of the buffer: the pointer
                // resting on words copies nothing.
                let text = self.buffer_text_shared();
                let token = self.symbol_token;
                Task::perform(
                    async move { crate::lsp::hover(&ext, &path, &text, line, col).await },
                    move |result| Message::SymbolHoverLoaded(token, at, result),
                )
            }
            Message::SymbolHoverLoaded(token, at, result) => {
                if let (true, Ok(Some(said))) = (token == self.symbol_token, result) {
                    self.symbol_card = Some((at, said));
                }
                Task::none()
            }
            Message::LargeMove(to) => {
                let (Some(path), Some(view)) = (self.current_file.clone(), self.large_view())
                else {
                    return Task::none();
                };
                let past_end = view.end >= view.size && to == LargeMove::Next;
                let before_start = view.start == 0 && to == LargeMove::Previous;
                if past_end || before_start {
                    self.set_status(t!("large.err_end"), false);
                    return Task::none();
                }
                Task::perform(
                    async move {
                        let moved = path.clone();
                        let result = tokio::task::spawn_blocking(move || match to {
                            LargeMove::First => file_io::read_window(&moved, 0, LARGE_WINDOW),
                            LargeMove::Next => file_io::read_window(&moved, view.end, LARGE_WINDOW),
                            LargeMove::Previous => {
                                file_io::read_window_before(&moved, view.start, LARGE_WINDOW)
                            }
                            LargeMove::Last => file_io::read_window(
                                &moved,
                                view.size.saturating_sub(LARGE_WINDOW),
                                LARGE_WINDOW,
                            ),
                        })
                        .await
                        .unwrap_or_else(|e| Err(e.to_string()));
                        (path, result)
                    },
                    |(path, result)| Message::LargeLoaded(path, result),
                )
            }
            Message::LargeLoaded(path, result) => {
                match result {
                    Ok((text, start, end)) if self.current_file.as_ref() == Some(&path) => {
                        if let Some(view) = self.large.get_mut(&path) {
                            view.start = start;
                            view.end = end;
                        }
                        self.replace_buffer_text(&text);
                        self.restore_cursor(0, 0);
                        self.undo_stack = History::default();
                        self.redo_stack = History::default();
                        self.is_dirty = false;
                        self.saved_fp = Some(fingerprint(&text));
                        self.text_rev += 1;
                    }
                    Ok(_) => {}
                    Err(e) => self.set_status(format!("{}: {e}", t!("status.error")), true),
                }
                Task::none()
            }
            Message::LargeRefreshed(path, result) => {
                if let Ok((text, start, end)) = result {
                    if self.current_file.as_ref() == Some(&path) {
                        let size = std::fs::metadata(&path).map_or(0, |m| m.len());
                        if let Some(view) = self.large.get_mut(&path) {
                            view.start = start;
                            view.end = end;
                            view.size = size;
                        }
                        // Read only: nothing to undo, the cursor stays.
                        if crate::editor::buffer_text(&self.editor.content) != text {
                            let (line, col) = self.editor.content.cursor_position();
                            self.replace_buffer_text(&text);
                            self.restore_cursor(line, col);
                            self.text_rev += 1;
                        }
                        self.saved_fp = Some(fingerprint(&text));
                    }
                }
                Task::none()
            }
            Message::LineOp(op) => {
                if self.editing_blocked() {
                    return Task::none();
                }
                let text = crate::editor::buffer_text(&self.editor.content);
                let span = self.selection_span();
                let (first, last) = self.lines_to_edit(span);
                let new_text = text_ops::line_op(&text, first, last, op);
                self.apply_line_edit(&text, new_text, 0, span)
            }
            Message::CaseOp(op) => {
                if self.editing_blocked() {
                    return Task::none();
                }
                if self.editor.content.selection().is_none() {
                    self.editor.content.perform(text_editor::Action::SelectWord);
                }
                let Some(selected) = self.editor.content.selection_all() else {
                    return Task::none();
                };
                let converted = text_ops::convert_case(&selected, op);
                if converted == selected {
                    return Task::none();
                }
                self.push_undo();
                self.undo_new_group = true;
                // Each selection converted on its own: one cursor's text never
                // lands at another.
                self.edited(|app| {
                    app.editor
                        .content
                        .map_selections(|text| text_ops::convert_case(text, op));
                });
                self.is_dirty = true;
                Task::none()
            }
            Message::ConvertIndent(to_spaces) => {
                if self.editing_blocked() {
                    return Task::none();
                }
                let text = crate::editor::buffer_text(&self.editor.content);
                let width = self.indent().1;
                let new_text = text_ops::convert_indent(&text, to_spaces, width);
                if let Some(path) = self.current_file.clone() {
                    self.indents.insert(path, (to_spaces, width));
                }
                let (line, col) = self.editor.content.cursor_position();
                self.apply_text_edit(new_text, line, col, None)
            }
            Message::OpenTextOptions(which) => {
                self.overlay = ActiveOverlay::TextOptions(which);
                Task::none()
            }
            Message::SetEncoding(encoding) => {
                if self.large_view().is_some() {
                    self.overlay = ActiveOverlay::None;
                    self.set_status(t!("large.read_only"), true);
                    return Task::none();
                }
                self.overlay = ActiveOverlay::None;
                let text = crate::editor::buffer_text(&self.editor.content);
                let mut format = self.format;
                format.encoding = encoding;
                if format.encode_bytes(&text).is_none() {
                    self.set_status(t!("status.err_encoding", name = encoding.label()), true);
                    return Task::none();
                }
                if self.format != format {
                    self.format = format;
                    self.is_dirty = true;
                }
                Task::none()
            }
            Message::RevertFile => {
                let Some(path) = self.current_file.clone().filter(|p| p.is_file()) else {
                    self.set_status(t!("status.err_no_file"), true);
                    return Task::none();
                };
                let too_large =
                    std::fs::metadata(&path).is_ok_and(|meta| meta.len() > MAX_OPEN_BYTES);
                if self.large_view().is_some() || too_large {
                    self.set_status(t!("large.read_only"), true);
                    return Task::none();
                }
                self.reload_later(path, self.format.encoding, true)
            }
            Message::ReloadEncoding(encoding) => {
                self.overlay = ActiveOverlay::None;
                let Some(path) = self.current_file.clone().filter(|p| p.is_file()) else {
                    self.set_status(t!("status.err_no_file"), true);
                    return Task::none();
                };
                if self.is_dirty {
                    self.set_status(t!("status.err_reload_dirty"), true);
                    return Task::none();
                }
                // A large file is shown in parts: reading it whole here would
                // freeze the window and could run out of memory.
                let too_large =
                    std::fs::metadata(&path).is_ok_and(|meta| meta.len() > MAX_OPEN_BYTES);
                if self.large_view().is_some() || too_large {
                    self.set_status(t!("large.read_only"), true);
                    return Task::none();
                }
                self.reload_later(path, encoding, false)
            }
            Message::SetLineEnding(ending) => {
                if self.large_view().is_some() {
                    self.overlay = ActiveOverlay::None;
                    self.set_status(t!("large.read_only"), true);
                    return Task::none();
                }
                self.overlay = ActiveOverlay::None;
                if self.format.ending != ending || self.format.mixed {
                    self.format.ending = ending;
                    // Chosen: every line takes it.
                    self.format.mixed = false;
                    self.is_dirty = true;
                }
                Task::none()
            }
            Message::SetIndent(spaces, width) => {
                self.overlay = ActiveOverlay::None;
                if let Some(path) = self.current_file.clone() {
                    self.indents.insert(path, (spaces, width));
                }
                Task::none()
            }
            Message::DetectIndent => {
                self.overlay = ActiveOverlay::None;
                if let Some(path) = self.current_file.clone() {
                    let text = crate::editor::buffer_text(&self.editor.content);
                    match text_ops::detect_indent(&text, self.config.tab_width) {
                        Some(found) => self.indents.insert(path, found),
                        None => self.indents.remove(&path),
                    };
                }
                Task::none()
            }
            Message::AddNextOccurrence(all) => {
                self.editor.content.add_next_occurrence(all);
                Task::none()
            }
            Message::AddCursor(down) => {
                self.editor
                    .content
                    .add_cursor_vertically(down, self.indent().1);
                Task::none()
            }
            Message::Zoom(step) => {
                self.config.font_size = if step == 0 {
                    Config::default().font_size
                } else {
                    (self.config.font_size + step as f32).clamp(8.0, 32.0)
                };
                // Written by the next tick: a turn of the wheel is many steps.
                self.config_dirty = true;
                Task::none()
            }
            Message::FindDirChosen(dir) => {
                self.finder.menu = None;
                self.handle(Message::FindDirPicked(Some(dir)))
            }
            Message::FindRecursive => {
                self.finder.recursive = !self.finder.recursive;
                self.search_files(false)
            }
            Message::FindMenu(menu) => {
                self.finder.menu = if self.finder.menu == menu { None } else { menu };
                Task::none()
            }
            Message::FindHistory(past) => {
                self.finder.multiline = past.contains('\n');
                self.finder_set_text(past);
                self.finder.menu = None;
                self.search_files(false)
            }
            Message::FindPin => {
                self.finder.pinned = !self.finder.pinned;
                Task::none()
            }
            Message::FindSelect(index) => self.finder_select(index),
            Message::FindExcludeAt(i) => {
                if let Some(o) = self.finder.hits.get_mut(i) {
                    o.excluded = !o.excluded;
                }
                Task::none()
            }
            Message::FindScrolled(offset) => {
                self.finder.scrolled = offset;
                Task::none()
            }
            Message::FindPreviewAction(action) => self.finder_preview_action(action),
            Message::FindPreviewMenu(what) => self.finder_preview_menu(what),
            Message::FinderPreviewRead(path, text) => {
                self.finder_preview_read(path, text);
                Task::none()
            }
            Message::ProjectsReloaded(loaded) => {
                self.projects_reloading = false;
                self.projects.reloaded(loaded);
                Task::none()
            }
            Message::RenameDone(outcome, disk) => self.rename_done(*outcome, disk),
            Message::ReplaceDone(replaced, disk) => self.replace_done(*replaced, disk),
            Message::FindPreviewPasted(text) => match text {
                Some(text) if text.len() <= MAX_PASTE_BYTES => self.finder_preview_action(
                    text_editor::Action::Edit(text_editor::Edit::Paste(std::sync::Arc::new(text))),
                ),
                _ => Task::none(),
            },
            Message::FindReplaceOne => self.finder_replace_one(),
            Message::FindReplaceAll => {
                self.finder.confirm_all = true;
                Task::none()
            }
            Message::FindReplaceConfirm(yes) => {
                if yes {
                    self.finder_replace_all()
                } else {
                    self.finder.confirm_all = false;
                    Task::none()
                }
            }
            Message::FindUndoReplace => self.undo_replacement(),
            Message::FindToWindow => self.finder_to_window(),
            Message::Results(msg) => self.results_update(msg),
            Message::Plugin(msg) => self.plugin_update(msg),
            Message::OpenTabList(at) => {
                self.tab_list = Some(at);
                Task::none()
            }
            Message::CloseTabList => {
                self.tab_list = None;
                Task::none()
            }
            Message::PickTab(path) => {
                self.tab_list = None;
                self.activate(path)
            }
            Message::TogglePanel(panel) => {
                // One panel at a time: the one asked for takes the place of
                // the other, and asking for the one that is open closes it.
                let was_open = match panel {
                    crate::sidebar::Panel::Files => self.panels.files,
                    crate::sidebar::Panel::Projects => self.panels.projects,
                };
                self.panels = crate::sidebar::Panels {
                    files: panel == crate::sidebar::Panel::Files && !was_open,
                    projects: panel == crate::sidebar::Panel::Projects && !was_open,
                };
                // A panel that is asked for shows the sidebar if it was hidden.
                if !was_open {
                    self.show_sidebar = true;
                    self.config.show_sidebar = true;
                    self.plugin_sidebar_yield();
                }
                self.config.show_files_panel = self.panels.files;
                self.config.show_projects_panel = self.panels.projects;
                self.config_dirty = true;
                Task::none()
            }
            Message::ToggleSidebar => {
                self.show_sidebar = !self.show_sidebar;
                self.config.show_sidebar = self.show_sidebar;
                self.config_dirty = true;
                Task::none()
            }
            Message::GitShortcut(what) => self.git_command(match what {
                GitShortcut::Commit => "git.commit",
                GitShortcut::Push => "git.push_options",
                GitShortcut::Pull => "git.pull",
            }),
            Message::OpenBranchSwitcher => self.git_command("git.branches"),
            Message::PrefsDragStart(grab) => {
                // From where it is: a window in the middle stays put while it
                // grows.
                let (x, y, w, h) = self.prefs_rect();
                self.prefs_pos = Some((x, y));
                self.prefs_size = Some((w, h));
                self.prefs_drag = Some(PrefsDrag {
                    grab,
                    from: None,
                    rect: (x, y, w, h),
                });
                Task::none()
            }
            Message::PrefsDragMove(point) => {
                if let Some(drag) = &mut self.prefs_drag {
                    // How far the pointer went from where it was at the first
                    // move.
                    let from = *drag.from.get_or_insert(point);
                    let (dx, dy) = (point.x - from.x, point.y - from.y);
                    let (x, y, w, h) = drag.rect;
                    use crate::preferences::PrefsGrab;
                    match drag.grab {
                        PrefsGrab::Move => self.prefs_pos = Some((x + dx, y + dy)),
                        PrefsGrab::Right => self.prefs_size = Some((w + dx, h)),
                        PrefsGrab::Bottom => self.prefs_size = Some((w, h + dy)),
                        PrefsGrab::Corner => self.prefs_size = Some((w + dx, h + dy)),
                    }
                }
                Task::none()
            }
            Message::PrefsDragEnd => {
                self.prefs_drag = None;
                Task::none()
            }
            Message::SidebarResizeStart => {
                self.sidebar_resizing = true;
                Task::none()
            }
            Message::SidebarResize(x) => {
                // The list starts after the bar, a gap from the window's edge
                // and a gap from the bar.
                let most = (self.config.window_width * 0.6).clamp(SIDEBAR_MIN, SIDEBAR_MAX);
                self.sidebar_width = (x - SIDEBAR_LEFT).clamp(SIDEBAR_MIN, most);
                Task::none()
            }
            Message::PanelResizeStart => {
                self.panel_resizing = true;
                self.panel_drag = None;
                Task::none()
            }
            Message::PanelResize(y) => {
                // Dragging up makes it taller: the height follows how far the
                // pointer went from where it was at the first move.
                let (y0, h0) = *self.panel_drag.get_or_insert((y, self.panel_height));
                let most = (self.config.window_height * 0.85).max(PANEL_MIN_HEIGHT);
                self.panel_height = (h0 + y0 - y).clamp(PANEL_MIN_HEIGHT, most);
                Task::none()
            }
            Message::PanelResizeEnd => {
                self.panel_resizing = false;
                self.panel_drag = None;
                Task::none()
            }
            Message::SidebarResizeEnd => {
                self.sidebar_resizing = false;
                self.config.sidebar_width = self.sidebar_width;
                self.config_dirty = true;
                Task::none()
            }
            Message::AskDeleteFile(path) => self.plugins_ask_delete(path),
            Message::DeleteFile(path) => {
                // The boxes of the plugins left ticked, by plugin.
                let mut chosen: std::collections::BTreeMap<String, Vec<String>> =
                    Default::default();
                if let ActiveOverlay::ConfirmFileDelete(asked, options) = &self.overlay {
                    if *asked == path {
                        for (plugin, option) in options.iter().filter(|(_, o)| o.checked) {
                            chosen
                                .entry(plugin.clone())
                                .or_default()
                                .push(option.id.clone());
                        }
                    }
                }
                self.overlay = ActiveOverlay::None;
                self.bookmarks.clear(&path);
                self.plugins_before_delete(&path, &chosen);
                let close = self.close_path(path.clone());
                let delete = Task::perform(delete_file(path), Message::FileDeleted);
                Task::batch([close, delete])
            }
            Message::FileDeleted(result) => {
                match result {
                    Ok(()) => self.set_status(t!("status.file_deleted"), false),
                    Err(e) => self.set_status(format!("{}: {e}", t!("status.error")), true),
                }
                Task::none()
            }
            Message::WindowFocus(focused) => {
                self.window_focused = focused;
                // What changed meanwhile is looked at right away.
                if focused {
                    return self.check_external_change();
                }
                Task::none()
            }
            Message::WindowResized(width, height) => {
                // Remembered, and written with the next autosave tick.
                if (self.config.window_width, self.config.window_height) != (width, height) {
                    self.config.window_width = width;
                    self.config.window_height = height;
                    self.config_dirty = true;
                }
                Task::none()
            }
            Message::TogglePreferences => {
                self.show_preferences = !self.show_preferences;
                Task::none()
            }
            Message::ToggleTheme => {
                let dark = !self.config.dark_mode;
                self.handle(Message::ThemeChanged(dark))
            }
            Message::ToggleHighContrast => {
                self.config.high_contrast = !self.config.high_contrast;
                se_theme::set_high_contrast(self.config.high_contrast);
                self.config_dirty = true;
                Task::none()
            }
            Message::ThemeChanged(dark) => {
                self.config.dark_mode = dark;
                self.config_dirty = true;
                Task::none()
            }
            Message::Sidebar(msg) => match self.sidebar.update(msg) {
                SidebarAction::OpenFile(path) => self.activate(path),
                SidebarAction::CloseFile(path) => self.request_close(path),
                SidebarAction::CloseNow(path) => self.request_close(path),
                // Only offered for the active file.
                SidebarAction::SaveFile => self.handle(Message::SaveFile),
                SidebarAction::SaveFileAs => self.start_save_as(self.current_file.clone()),
                SidebarAction::None => Task::none(),
            },
            Message::Search(SearchMessage::ReplaceAll | SearchMessage::Replace)
                if self.large_view().is_some() =>
            {
                self.set_status(t!("large.read_only"), true);
                Task::none()
            }
            Message::Search(SearchMessage::ReplaceAll) => {
                let old = crate::editor::buffer_text(&self.editor.content);
                match self.search.replace_all(&old) {
                    Ok(new_text) if new_text != old => {
                        self.push_undo();
                        self.undo_new_group = true;
                        self.is_dirty = true;
                        self.search.reset_matches();
                        // Minimal replacement: scroll and cursor survive.
                        self.replace_buffer_text(&new_text);
                    }
                    Ok(_) => self.search.reset_matches(),
                    Err(e) => self.search.set_error(e),
                }
                Task::none()
            }
            Message::Search(msg) => {
                let replacing = matches!(msg, SearchMessage::Replace);
                let before = replacing.then(|| self.snapshot());
                let probe = replacing.then(|| self.edit_probe());
                let command = self.search.update(msg, &mut self.editor.content);
                // Replacements are edits like any other: undoable and dirty.
                if let (Some(before), Some((count, top))) = (before, probe) {
                    if crate::editor::buffer_text(&self.editor.content) != before.text {
                        self.undo_stack.push_back(before);
                        trim_history(&mut self.undo_stack);
                        self.redo_stack.clear();
                        self.undo_new_group = true;
                        self.is_dirty = true;
                        self.shift_bookmarks(count, top);
                    }
                }
                command
            }
            Message::Preferences(msg) => {
                // A language's completion setting: its servers start again
                // with it.
                if matches!(
                    &msg,
                    PreferencesMessage::CompletionOption(..)
                        | PreferencesMessage::ResetGroup(crate::preferences::Group::Completion)
                ) {
                    let _ = self.preferences.update(msg, &mut self.config);
                    self.config_dirty = true;
                    crate::lsp::set_completion_options(&self.config.completion_options);
                    self.completion_servers_restarted();
                    return Task::future(crate::lsp::restart_servers()).discard();
                }
                // The Lint page shows the settings of the Lint plugin, the page
                // of a plugin its own.
                if matches!(
                    msg,
                    PreferencesMessage::PageSelected(crate::preferences::PrefsPage::Lint)
                ) {
                    let _ = self.preferences.update(msg, &mut self.config);
                    return self.plugin_lint_settings_open();
                }
                if let PreferencesMessage::PluginPageSelected(id) = &msg {
                    let id = id.clone();
                    let _ = self.preferences.update(msg, &mut self.config);
                    let spelling = self
                        .plugins
                        .get(&id)
                        .is_some_and(|p| p.manifest.contributes.speller.is_some());
                    let dictionaries = if spelling {
                        self.load_spell_choices()
                    } else {
                        Task::none()
                    };
                    return Task::batch([self.plugin_settings_show(&id), dictionaries]);
                }
                let servers_page = matches!(
                    msg,
                    PreferencesMessage::PageSelected(crate::preferences::PrefsPage::Servers)
                );
                // The Plugins page: the list of the official ones and the
                // updates are looked up when it is shown.
                if matches!(
                    msg,
                    PreferencesMessage::PageSelected(crate::preferences::PrefsPage::Plugins)
                ) {
                    let _ = self.preferences.update(msg, &mut self.config);
                    let list =
                        if matches!(self.plugins.registry, crate::plugins::Registry::Loaded(_)) {
                            Task::none()
                        } else {
                            self.plugin_update(crate::plugins::PluginMsg::LoadRegistry)
                        };
                    let updates = self.plugin_update(crate::plugins::PluginMsg::CheckUpdates);
                    return Task::batch([list, updates]);
                }
                if self.preferences.update(msg, &mut self.config) {
                    self.config_dirty = true;
                }
                if servers_page {
                    return Task::perform(
                        async {
                            tokio::task::spawn_blocking(|| {
                                (crate::lsp_install::ready(), crate::lsp_install::removable())
                            })
                            .await
                        },
                        |servers| Message::ServersChecked(servers.unwrap_or_default()),
                    );
                }
                Task::none()
            }
            Message::About => {
                self.overlay = ActiveOverlay::About;
                Task::none()
            }
            // A second window doesn't keep the session: its unsaved work
            // would be gone.
            Message::Quit
                if !self.session_owner
                    && self.sidebar.files().iter().any(|p| self.is_path_dirty(p)) =>
            {
                self.overlay = ActiveOverlay::ConfirmQuit(t!("quit.unsaved").to_string());
                Task::none()
            }
            // Edits waiting in a tab of a plugin or a merge tab are not in
            // the session: closing would throw them away.
            Message::Quit
                if self.plugins.tabs.values().any(|t| t.dirty)
                    || self.merge_tabs.values().any(|t| t.is_modified()) =>
            {
                self.overlay = ActiveOverlay::ConfirmQuit(t!("quit.unsaved").to_string());
                Task::none()
            }
            Message::Quit => match self.save_session_now() {
                Ok(()) => {
                    // A second window would undo the settings changed meanwhile
                    // in the first.
                    if self.session_owner {
                        self.config.save();
                    }
                    crate::lsp::stop_servers();
                    crate::plugins::host::stop_all();
                    std::process::exit(0)
                }
                // Unsaved buffers live on in the session file; if it can't be
                // written, quitting would lose them.
                Err(e) => {
                    self.overlay = ActiveOverlay::ConfirmQuit(e);
                    Task::none()
                }
            },
            Message::QuitAnyway => {
                if self.session_owner {
                    self.config.save();
                }
                crate::lsp::stop_servers();
                crate::plugins::host::stop_all();
                std::process::exit(0)
            }
            Message::ContextCopy => {
                if let Some(text) = self.editor.content.selection() {
                    return iced::clipboard::write(text);
                }
                Task::none()
            }
            Message::ContextCut => {
                if self.editing_blocked() {
                    return Task::none();
                }
                if let Some(text) = self.editor.content.selection() {
                    self.push_undo();
                    self.undo_new_group = true;
                    self.edited(|app| {
                        app.editor
                            .content
                            .perform(text_editor::Action::Edit(text_editor::Edit::Delete));
                    });
                    self.is_dirty = true;
                    return iced::clipboard::write(text);
                }
                Task::none()
            }
            Message::ContextPaste => {
                if self.editing_blocked() {
                    return Task::none();
                }
                iced::clipboard::read().map(Message::ClipboardContent)
            }
            Message::ClipboardContent(Some(text)) => {
                if self.editing_blocked() {
                    return Task::none();
                }
                if text.len() > MAX_PASTE_BYTES {
                    self.set_status(
                        format!(
                            "{}: {}",
                            t!("status.error"),
                            t!("status.err_paste_too_large")
                        ),
                        true,
                    );
                    return Task::none();
                }
                self.push_undo();
                self.undo_new_group = true;
                self.edited(|app| {
                    app.editor.content.perform(text_editor::Action::Edit(
                        text_editor::Edit::Paste(std::sync::Arc::new(text)),
                    ));
                });
                self.is_dirty = true;
                Task::none()
            }
            Message::ClipboardContent(None) => Task::none(),
            Message::ContextDelete => {
                // Nothing selected, nothing to delete (and no empty undo step).
                if self.editing_blocked() || self.editor.content.selection().is_none() {
                    return Task::none();
                }
                self.push_undo();
                self.undo_new_group = true;
                self.edited(|app| {
                    app.editor
                        .content
                        .perform(text_editor::Action::Edit(text_editor::Edit::Delete));
                });
                self.is_dirty = true;
                Task::none()
            }
            Message::FormatFile | Message::FormatSelection if self.large_view().is_some() => {
                self.format_on_save_pending = false;
                self.set_status(t!("large.read_only"), true);
                Task::none()
            }
            Message::FormatFile => {
                if self.is_formatting {
                    return Task::none();
                }
                if let Some(ext) = self.editor.language.clone() {
                    if !crate::formatter::can_format(&ext) {
                        self.format_on_save_pending = false;
                        self.set_status(t!("status.err_no_formatter"), true);
                        return Task::none();
                    }
                    self.is_formatting = true;
                    self.set_status(t!("status.formatting"), false);
                    let content = crate::editor::buffer_text(&self.editor.content);
                    let job = FormatJob {
                        path: self.current_file.clone(),
                        source: fingerprint(&content),
                    };
                    let target = self.format_target();
                    let (spaces, width) = self.indent();
                    Task::perform(
                        async move {
                            if crate::formatter::has_formatter(&ext) {
                                crate::formatter::format(content, ext, target).await
                            } else if let Some(path) = target {
                                crate::lsp::format(&ext, &path, &content, spaces, width).await
                            } else {
                                Err(t!("status.err_no_formatter").to_string())
                            }
                        },
                        move |result| Message::FileFormatted(job.clone(), result),
                    )
                } else {
                    self.set_status(t!("status.no_language"), false);
                    Task::none()
                }
            }
            Message::FormatSelection => {
                if self.is_formatting {
                    return Task::none();
                }
                if let (Some(ext), Some(selected)) = (
                    self.editor.language.clone(),
                    self.editor.content.selection(),
                ) {
                    self.is_formatting = true;
                    self.set_status(t!("status.formatting"), false);
                    let job = FormatJob {
                        path: self.current_file.clone(),
                        source: fingerprint(&selected),
                    };
                    let target = self.format_target();
                    // Formatters end their output with a newline; a fragment
                    // that didn't end with one shouldn't gain one.
                    let had_newline = selected.ends_with('\n');
                    // A language with only a language server: it formats the
                    // range, in the file.
                    let by_server = if crate::formatter::has_formatter(&ext) {
                        None
                    } else {
                        target.clone().zip(self.editor.content.selection_range())
                    };
                    let content = crate::editor::buffer_text(&self.editor.content);
                    let (spaces, width) = self.indent();
                    Task::perform(
                        async move {
                            if let Some((path, (start, end))) = by_server {
                                return crate::lsp::format_range(
                                    &ext, &path, &content, start, end, spaces, width,
                                )
                                .await;
                            }
                            crate::formatter::format(selected, ext, target)
                                .await
                                .map(|f| {
                                    if had_newline {
                                        f
                                    } else {
                                        f.trim_end_matches('\n').to_string()
                                    }
                                })
                        },
                        move |result| Message::SelectionFormatted(job.clone(), result),
                    )
                } else {
                    Task::none()
                }
            }
            Message::FileFormatted(job, result) => {
                let save_after = std::mem::take(&mut self.format_on_save_pending);
                let task = self.file_formatted(job, result);
                if save_after {
                    self.skip_format_once = true;
                    Task::batch([task, self.handle(Message::SaveFile)])
                } else {
                    task
                }
            }
            Message::SelectionFormatted(job, result) => {
                self.is_formatting = false;
                let current = self.current_file == job.path
                    && self
                        .editor
                        .content
                        .selection()
                        .is_some_and(|sel| fingerprint(&sel) == job.source);
                if !current {
                    self.set_status(t!("status.format_stale"), true);
                    return Task::none();
                }
                match result {
                    Ok(formatted) => {
                        self.push_undo();
                        self.undo_new_group = true;
                        self.edited(|app| {
                            app.editor.content.perform(text_editor::Action::Edit(
                                text_editor::Edit::Paste(std::sync::Arc::new(formatted)),
                            ));
                        });
                        self.is_dirty = true;
                        self.format_error = None;
                        self.show_error_panel = false;
                        self.set_status(t!("status.formatted"), false);
                    }
                    Err(e) => self.show_format_error(e),
                }
                Task::none()
            }
            Message::CloseErrorPanel => {
                self.show_error_panel = false;
                Task::none()
            }
            Message::Undo if self.on_diff_tab() => self.undo_work(true),
            Message::Redo if self.on_diff_tab() => self.undo_work(false),
            Message::Undo if self.on_merge_tab() => {
                self.merge_msg(crate::diff_view::merge::MergeMsg::Undo)
            }
            Message::Redo if self.on_merge_tab() => {
                self.merge_msg(crate::diff_view::merge::MergeMsg::Redo)
            }
            Message::Undo => {
                if self.editing_blocked() {
                    return Task::none();
                }
                if let Some(prev) = self.undo_stack.pop_back() {
                    let current = self.snapshot();
                    self.redo_stack.push_back(current);
                    trim_history(&mut self.redo_stack);
                    return self.restore_snapshot(prev);
                }
                Task::none()
            }
            Message::Redo => {
                if self.editing_blocked() {
                    return Task::none();
                }
                if let Some(next) = self.redo_stack.pop_back() {
                    let current = self.snapshot();
                    self.undo_stack.push_back(current);
                    trim_history(&mut self.undo_stack);
                    return self.restore_snapshot(next);
                }
                Task::none()
            }
            Message::AutoSave => {
                let scratch = self.flush_scratch_later();
                if self.config_dirty {
                    self.config_dirty = false;
                    self.config.save();
                }
                // The tree follows what happens on the disk.
                // Not while the window is in the background.
                let watching = self.window_focused;
                // What is watched: the projects, and the folders of the open
                // files that are in none.
                if self.ticks.is_multiple_of(5) {
                    self.sync_watch();
                }
                // Told by the watcher: looked at when something changed (and
                // now and then, in case an event was lost); without a
                // watcher, at intervals.
                let driven = self.watch.active();
                let changed = std::mem::take(&mut self.fs_dirty);
                let every = |n: u32, ticks: u32| {
                    if driven {
                        changed || ticks.is_multiple_of(30)
                    } else {
                        ticks.is_multiple_of(n)
                    }
                };
                let mut reload = Task::none();
                if watching && every(2, self.ticks) {
                    reload = self.reload_projects();
                }
                let external = if watching && (!driven || changed || self.ticks.is_multiple_of(30))
                {
                    self.check_external_change()
                } else {
                    Task::none()
                };
                self.ticks = self.ticks.wrapping_add(1);
                self.refresh_marks_now();
                let external = Task::batch([external, self.auto_save_files(), scratch, reload]);
                // The session is built (on this thread) every third tick at
                // most.
                // On a big buffer the copy of its text waits for a pause in the
                // typing.
                let typing_in_big = self.editor.content.line_count() > 40_000
                    && self.last_edit.elapsed() < std::time::Duration::from_secs(3);
                if self.rev == self.session_rev || !self.ticks.is_multiple_of(3) || typing_in_big {
                    return external;
                }
                match self.session_write_job() {
                    Some(job) => {
                        Task::batch([external, Task::perform(job, Message::SessionWritten)])
                    }
                    None => external,
                }
            }
            Message::FsChanged => {
                self.fs_dirty = true;
                // A folder just made is watched now, not at the next sync:
                // what is created in it straight away would be missed.
                self.watch.watch_created();
                Task::none()
            }
            Message::SessionWritten(result) => {
                match result {
                    Ok(()) => {}
                    Err(e) => {
                        // Try again at the next tick.
                        self.last_session
                            .store(0, std::sync::atomic::Ordering::Relaxed);
                        self.session_rev = 0;
                        self.set_status(format!("{}: {}", t!("status.session_not_saved"), e), true);
                    }
                }
                Task::none()
            }
            Message::PreviewTick => {
                self.refresh_preview();
                Task::none()
            }
            // --- Overlays ---
            Message::CloseOverlay => {
                if self.overlay_is_dismissable() {
                    self.dismiss_plugin_question();
                    self.capturing = None;
                    self.overlay = ActiveOverlay::None;
                }
                Task::none()
            }
            Message::OpenGotoLine => {
                self.goto_line_input = String::new();
                self.overlay = ActiveOverlay::GotoLine;
                iced::widget::operation::focus(goto_line_id())
            }
            Message::GotoLineInputChanged(s) => {
                // Allow only digits
                self.goto_line_input = s.chars().filter(|c| c.is_ascii_digit()).collect();
                Task::none()
            }
            Message::GotoLineSubmit => {
                if let Ok(n) = self.goto_line_input.parse::<usize>() {
                    let max = self.editor.content.line_count().saturating_sub(1);
                    let target = n.saturating_sub(1).min(max);
                    self.remember_place();
                    crate::search::goto_line_start(&mut self.editor.content, target);
                }
                self.overlay = ActiveOverlay::None;
                Task::none()
            }
            Message::OpenLanguagePicker => {
                self.overlay = ActiveOverlay::LanguagePicker;
                Task::none()
            }
            Message::SetLanguage(ext) => {
                self.editor.language = if ext.is_empty() { None } else { Some(ext) };
                self.reset_view_mode();
                self.overlay = ActiveOverlay::None;
                Task::none()
            }
            // The shortcuts are set in the Preferences, on their page.
            Message::OpenUrl(url) => {
                plugins::open_url(url);
                Task::none()
            }
            Message::OpenShortcuts => {
                self.capturing = None;
                self.overlay = ActiveOverlay::None;
                self.show_preferences = true;
                self.handle(Message::Preferences(PreferencesMessage::PageSelected(
                    crate::preferences::PrefsPage::Shortcuts,
                )))
            }
            Message::StartCaptureShortcut(target) => {
                self.capturing = Some(target);
                Task::none()
            }
            Message::OpenPalette => self.open_everywhere(crate::everywhere::Tab::Actions),
            Message::OpenEverywhere(tab) => self.open_everywhere(tab),
            Message::DoubleShift => {
                if self.capturing.is_some() {
                    return Task::none();
                }
                if self.overlay == ActiveOverlay::Everywhere {
                    return self.everywhere_non_project();
                }
                if !matches!(self.overlay, ActiveOverlay::None) && !self.overlay_is_dismissable() {
                    return Task::none();
                }
                self.open_everywhere(crate::everywhere::Tab::All)
            }
            Message::EverywhereQuery(query) => self.everywhere_typed(query),
            Message::EverywhereTab(tab) => self.everywhere_switch(tab),
            Message::EverywhereOpen(pick) => self.everywhere_open(pick),
            Message::EverywhereFiles(generation, files) => self.everywhere_files(generation, files),
            Message::EverywhereSymbolsDue(generation) => self.everywhere_symbols_due(generation),
            Message::EverywhereSymbols(generation, found) => {
                self.everywhere_symbols(generation, found)
            }
            Message::EverywhereTextDue(generation) => self.everywhere_text_due(generation),
            Message::EverywhereText(generation, found) => {
                self.everywhere_text(generation, found);
                Task::none()
            }
            Message::EverywhereNonProject => self.everywhere_non_project(),
            Message::EverywhereFilter => {
                self.everywhere.filter_open = !self.everywhere.filter_open;
                Task::none()
            }
            Message::EverywhereFilterType(kind, on) => self.everywhere_filter_type(kind, on),
            Message::EverywhereTextOption(option) => self.everywhere_text_option(option),
            Message::EverywhereExport => self.everywhere_export(),
            Message::EverywhereAbbreviate(command) => self.everywhere_abbreviate(command),
            Message::EverywhereAbbreviationTyped(text) => {
                if let Some((_, typed)) = self.everywhere.abbreviating.as_mut() {
                    *typed = text;
                }
                Task::none()
            }
            Message::EverywhereAbbreviationSubmit => self.everywhere_abbreviation_submit(),
            Message::EverywhereAbbreviationRemove(command, abbreviation) => {
                self.everywhere_abbreviation_remove(command, abbreviation);
                Task::none()
            }
            Message::EverywhereAssign(command) => self.everywhere_assign(command),
            // Raw key forwarded from the subscription fn pointer
            Message::KeyPressed {
                key,
                ctrl,
                shift,
                alt,
            } => {
                // In shortcut capture mode: save the new binding
                if let Some(target) = self.capturing {
                    let sc = ShortcutConfig {
                        ctrl,
                        shift,
                        alt,
                        key,
                    };
                    // A bare letter (or Shift+letter) would fire on every
                    // keystroke typed into the editor.
                    if !(sc.ctrl || sc.alt || crate::config::is_function_key(&sc.key)) {
                        let ctrl = if cfg!(target_os = "macos") {
                            "⌘"
                        } else {
                            "Ctrl"
                        };
                        self.set_status(t!("shortcuts.needs_modifier", ctrl = ctrl), true);
                        return Task::none();
                    }
                    if crate::code_editor::consumes_key(&sc.key, sc.ctrl, sc.alt) {
                        self.set_status(t!("shortcuts.reserved"), true);
                        return Task::none();
                    }
                    if self.config.shortcuts.conflicts(target, &sc) {
                        self.set_status(t!("shortcuts.conflict"), true);
                        return Task::none();
                    }
                    *self.config.shortcuts.slot_mut(target) = sc;
                    self.config.save();
                    self.capturing = None;
                    return Task::none();
                }
                // Typing in the result of a merge: the app's shortcuts stay
                // out of it.
                if self.merge_typing {
                    // Going back in a merge result is still the user's own.
                    if self.on_merge_tab() && ctrl && key.eq_ignore_ascii_case("z") {
                        return self.handle(if shift { Message::Redo } else { Message::Undo });
                    }
                    if self.on_merge_tab() && ctrl && key.eq_ignore_ascii_case("y") {
                        return self.handle(Message::Redo);
                    }
                    let toggle = &self.config.shortcuts.toggle_git;
                    if toggle.key.eq_ignore_ascii_case(&key)
                        && toggle.ctrl == ctrl
                        && toggle.alt == alt
                        && toggle.shift == shift
                    {
                        return self.handle(Message::ToggleGit);
                    }
                    return Task::none();
                }
                // The terminal has the keyboard: it gets every key (it reads
                // them itself), except the one that hides it.
                if self.plugins.screen_focus.is_some() {
                    let toggle = &self.config.shortcuts.toggle_terminal;
                    if toggle.key.eq_ignore_ascii_case(&key)
                        && toggle.ctrl == ctrl
                        && toggle.alt == alt
                        && toggle.shift == shift
                    {
                        return self.handle(Message::ToggleTerminal);
                    }
                    return Task::none();
                }
                // The completions of an editor of a panel, then the grid of
                // a panel that has the keyboard.
                if !ctrl && !alt {
                    if let Some(task) = self.plugin_completion_key(&key) {
                        return task;
                    }
                }
                if let Some(task) = self.plugin_grid_key(&key, ctrl, alt, shift) {
                    return task;
                }
                if let Some(task) = self.plugin_panel_key(&key, ctrl, alt, shift) {
                    return task;
                }
                // The list of completions: its arrows, pages, Enter and Tab.
                if self.completion_visible() && !ctrl && !alt {
                    if let Some(task) = self.completion_key(&key) {
                        return task;
                    }
                }
                // Alt+↑ / Alt+↓: another signature of an overloaded call.
                if alt
                    && !ctrl
                    && !shift
                    && (key == "ArrowUp" || key == "ArrowDown")
                    && self.hint_cycle(if key == "ArrowDown" { 1 } else { -1 })
                {
                    return Task::none();
                }
                // Tab in a snippet: its next place.
                if key == "Tab" && !ctrl && !alt && !shift && self.snippet_next() {
                    return Task::none();
                }
                // Tab after the trigger of a snippet: the snippet.
                if key == "Tab" && !ctrl && !alt && !shift && self.overlay == ActiveOverlay::None {
                    if let Some(task) = self.snippet_expand_trigger() {
                        return task;
                    }
                }
                // Search everywhere: its keys; the shortcut of a tab goes to
                // that tab (again: beyond the project).
                if self.overlay == ActiveOverlay::Everywhere {
                    if let Some(task) = self.everywhere_key(&key, ctrl, shift, alt) {
                        return task;
                    }
                    let sh = &self.config.shortcuts;
                    let is = |s: &ShortcutConfig| {
                        s.key.eq_ignore_ascii_case(&key)
                            && s.ctrl == ctrl
                            && s.alt == alt
                            && s.shift == shift
                    };
                    use crate::everywhere::Tab;
                    let tab = [
                        (&sh.goto_file, Tab::Files),
                        (&sh.goto_class, Tab::Classes),
                        (&sh.goto_symbol, Tab::Symbols),
                        (&sh.command_palette, Tab::Actions),
                    ]
                    .into_iter()
                    .find(|(s, _)| is(s))
                    .map(|(_, t)| t);
                    if let Some(tab) = tab {
                        return self.open_everywhere(tab);
                    }
                    return Task::none();
                }
                // The small lists (language, encoding, tasks, spelling…): the
                // arrows go through them and Enter takes the row.
                let choices = self.overlay_choices();
                if !choices.is_empty() && !ctrl && !alt {
                    match key.as_str() {
                        "ArrowDown" => {
                            self.overlay_pick = (self.overlay_pick + 1) % choices.len();
                            return Task::none();
                        }
                        "ArrowUp" => {
                            self.overlay_pick =
                                (self.overlay_pick + choices.len() - 1) % choices.len();
                            return Task::none();
                        }
                        "Return" => {
                            let message = choices[self.overlay_pick.min(choices.len() - 1)].clone();
                            return self.handle(message);
                        }
                        _ => {}
                    }
                }
                // The finder cards: arrows move in the list.
                if self.overlay == ActiveOverlay::FindInFiles {
                    return match (key.as_str(), ctrl, alt) {
                        // Alt+↓: the searches made.
                        ("ArrowDown", false, true) => {
                            self.handle(Message::FindMenu(Some(finder::Menu::History)))
                        }
                        // On several lines the arrows are the text's:
                        // Ctrl+Alt+↑/↓ go through the matches.
                        ("ArrowDown", true, true) => self.finder_step(1),
                        ("ArrowUp", true, true) => self.finder_step(-1),
                        ("ArrowDown", false, false) if !self.finder.multiline => {
                            self.finder_step(1)
                        }
                        ("ArrowUp", false, false) if !self.finder.multiline => self.finder_step(-1),
                        // Ctrl+Enter: to the Results panel.
                        ("Return", true, false) => self.finder_to_window(),
                        // Alt+Delete: leave the match out of "Replace all" (or put it back).
                        ("Delete", false, true) => {
                            self.finder_toggle_exclude();
                            Task::none()
                        }
                        _ => Task::none(),
                    };
                }
                // The usages list: arrows move, Enter opens the highlighted one.
                if let ActiveOverlay::References(list) = &mut self.overlay {
                    match key.as_str() {
                        "ArrowDown" | "ArrowUp" => {
                            let delta = if key == "ArrowDown" { 1 } else { -1 };
                            list.selected =
                                crate::palette::step(list.selected, delta, list.items.len());
                            return views::scroll_references_to(list.selected);
                        }
                        "Return" => {
                            let selected = list.selected;
                            return self.handle(Message::OpenReference(selected));
                        }
                        _ => {}
                    }
                }
                // Don't fire shortcuts while any overlay or the Preferences
                // window is open
                if !matches!(self.overlay, ActiveOverlay::None) || self.show_preferences {
                    return Task::none();
                }
                // Ctrl+Alt+↑/↓: the previous or next match of the Results.
                if ctrl
                    && alt
                    && !shift
                    && (key == "ArrowUp" || key == "ArrowDown")
                    && !self.results.tabs.is_empty()
                {
                    return self.results_update(results::ResultsMsg::Step(key == "ArrowDown"));
                }
                // The Results panel, clicked last: its keys.
                if let Some(task) = self.results_key(&key, ctrl, shift, alt) {
                    return task;
                }
                // The tree, clicked last: the arrows move in it, Enter opens.
                if self.tree_focus && !ctrl && !alt && self.show_sidebar && self.panels.projects {
                    match key.as_str() {
                        "ArrowUp" | "ArrowDown" | "ArrowLeft" | "ArrowRight" => {
                            self.projects.step(&key);
                            self.save_projects();
                            return self.scroll_tree_to_selected();
                        }
                        "Return" => {
                            let Some(path) = self.projects.selected.clone() else {
                                return Task::none();
                            };
                            if path.is_dir() {
                                self.projects.toggle(&path);
                                self.save_projects();
                                return Task::none();
                            }
                            // Enter keeps the file open, as a second click does.
                            if self.preview_tab.as_ref() == Some(&path) {
                                self.preview_tab = None;
                            }
                            return self.activate(path);
                        }
                        _ => {}
                    }
                }
                // On a diff, Ctrl+A picks every row and Ctrl+C / Ctrl+X copy the
                // picked ones (a diff can't be cut from).
                if ctrl && !alt && !shift {
                    if let Some(task) = self.diff_clipboard(&key) {
                        return task;
                    }
                }
                // Enter in the search card goes to the next match, Shift+Enter
                // to the previous one — when the find field has the focus
                // (it captures the key itself, which is why this listens to
                // captured events too).
                if key == "Return" && !ctrl && !alt {
                    if self.show_search {
                        return iced::advanced::widget::operate(crate::search::find_field_focused()).map(
                            move |in_find_field| Message::SearchEnterChecked {
                                shift,
                                in_find_field,
                            },
                        );
                    }
                    return Task::none();
                }
                // Tab indents, Shift+Tab dedents; a search field keeps it.
                if key == "Tab" {
                    // Ctrl+Tab goes to the tab seen before (again for older
                    // ones while Ctrl is held), Ctrl+Shift+Tab back.
                    if ctrl && !alt {
                        return self.switch_tab(!shift);
                    }
                    if ctrl || alt {
                        return Task::none();
                    }
                    let m = if shift {
                        Message::DedentLine
                    } else {
                        Message::InsertTab
                    };
                    return self.check_search_focus_then(m);
                }
                // Match against config shortcuts. A symbol key (`/`) may need
                // Shift on some layouts (AZERTY), so Shift doesn't count
                // against a binding that doesn't use it.
                let symbol_key = key.chars().count() == 1
                    && key.chars().next().is_some_and(|c| !c.is_alphanumeric());
                let sc_check = |s: &ShortcutConfig| -> bool {
                    s.key.eq_ignore_ascii_case(&key)
                        && s.ctrl == ctrl
                        && s.alt == alt
                        && (s.shift == shift || (symbol_key && !s.shift))
                };
                let sh = &self.config.shortcuts;
                // Order matters: Ctrl+Shift+S must be tried before Ctrl+S.
                let bindings: [(&ShortcutConfig, Message); 46] = [
                    (&sh.spell_fix, Message::SpellFix),
                    (&sh.run_task, Message::OpenTaskPicker),
                    (&sh.rename_symbol, Message::RenameSymbol),
                    (&sh.add_next_occurrence, Message::AddNextOccurrence(false)),
                    (&sh.select_all_occurrences, Message::AddNextOccurrence(true)),
                    (&sh.add_cursor_above, Message::AddCursor(false)),
                    (&sh.add_cursor_below, Message::AddCursor(true)),
                    (&sh.complete, Message::CompletionInvoke),
                    (&sh.parameter_info, Message::HintInvoke),
                    (&sh.toggle_git, Message::ToggleGit),
                    (&sh.git_commit, Message::GitShortcut(GitShortcut::Commit)),
                    (&sh.git_push, Message::GitShortcut(GitShortcut::Push)),
                    (&sh.git_pull, Message::GitShortcut(GitShortcut::Pull)),
                    (&sh.git_branches, Message::OpenBranchSwitcher),
                    (&sh.bookmark_panel, Message::ToggleBookmarkPanel),
                    (&sh.toggle_terminal, Message::ToggleTerminal),
                    (&sh.nav_back, Message::NavBack),
                    (&sh.nav_forward, Message::NavForward),
                    (&sh.undo, Message::Undo),
                    (&sh.redo, Message::Redo),
                    (&sh.duplicate_line, Message::DuplicateLine),
                    (&sh.move_line_up, Message::MoveLineUp),
                    (&sh.move_line_down, Message::MoveLineDown),
                    (&sh.toggle_comment, Message::ToggleComment),
                    (&sh.delete_line, Message::DeleteLine),
                    (&sh.toggle_bookmark, Message::ToggleBookmark),
                    (&sh.prev_bookmark, Message::PrevBookmark),
                    (&sh.next_bookmark, Message::NextBookmark),
                    (&sh.toggle_fold, Message::ToggleFold),
                    (&sh.save_as, Message::SaveFileAs),
                    (&sh.new_file, Message::NewFile),
                    (&sh.save_file, Message::SaveFile),
                    (&sh.open_file, Message::OpenFile),
                    (&sh.close_file, Message::CloseFile),
                    (&sh.find, Message::ToggleSearch),
                    (&sh.select_all, Message::SelectAll),
                    (&sh.format_code, Message::FormatFile),
                    (&sh.goto_line, Message::OpenGotoLine),
                    (&sh.command_palette, Message::OpenPalette),
                    (&sh.toggle_sidebar, Message::ToggleSidebar),
                    (&sh.goto_file, Message::OpenGotoFile),
                    (
                        &sh.goto_class,
                        Message::OpenEverywhere(crate::everywhere::Tab::Classes),
                    ),
                    (
                        &sh.goto_symbol,
                        Message::OpenEverywhere(crate::everywhere::Tab::Symbols),
                    ),
                    (&sh.find_in_files, Message::OpenFindInFiles),
                    (&sh.replace_in_files, Message::OpenReplaceInFiles),
                    (&sh.reopen_tab, Message::ReopenTab),
                ];
                let msg = bindings
                    .into_iter()
                    .find(|(s, _)| sc_check(s))
                    .map(|(_, m)| m)
                    .or_else(|| sc_check(&sh.quit).then_some(Message::Quit));
                match msg {
                    Some(m) => self.check_search_focus_then(m),
                    // Ctrl and + / - / 0: the size of the text.
                    None if ctrl && !alt => match key.as_str() {
                        "=" | "+" => self.handle(Message::Zoom(1)),
                        "-" => self.handle(Message::Zoom(-1)),
                        "0" => self.handle(Message::Zoom(0)),
                        _ => Task::none(),
                    },
                    None => Task::none(),
                }
            }
            Message::DuplicateLine => {
                if self.editing_blocked() {
                    return Task::none();
                }
                let text = crate::editor::buffer_text(&self.editor.content);
                let span = self.selection_span();
                let (first, last) = self.lines_to_edit(span);
                let new_text = text_ops::duplicate_lines(&text, first, last);
                // The copy goes below, and the cursor follows it.
                let shift = (last - first + 1) as isize;
                self.apply_line_edit(&text, new_text, shift, span)
            }
            Message::MoveLineUp => self.move_lines(true),
            Message::MoveLineDown => self.move_lines(false),
            Message::ToggleComment => {
                if self.editing_blocked() {
                    return Task::none();
                }
                // Plain text and formats without comments (JSON) have nothing
                // to toggle.
                let Some((prefix, suffix)) = comment_style(self.editor.language.as_deref()) else {
                    return Task::none();
                };
                let text = crate::editor::buffer_text(&self.editor.content);
                let span = self.selection_span();
                let (first, last) = self.lines_to_edit(span);
                let new_text = text_ops::toggle_comment_lines(&text, first, last, prefix, suffix);
                self.apply_line_edit(&text, new_text, 0, span)
            }
            Message::DeleteLine => {
                if self.editing_blocked() {
                    return Task::none();
                }
                let text = crate::editor::buffer_text(&self.editor.content);
                let (first, last) = self.lines_to_edit(self.selection_span());
                let (new_text, new_line) = text_ops::delete_lines(&text, first, last);
                self.apply_text_edit(new_text, new_line, 0, None)
            }
            Message::ToggleBookmark => {
                let Some(path) = self.current_file.clone() else {
                    return Task::none();
                };
                let (line, _) = self.editor.content.cursor_position();
                let added = self.bookmarks.toggle(&path, line);
                self.status_message = if added {
                    t!("status.bookmark_added")
                } else {
                    t!("status.bookmark_removed")
                }
                .to_string();
                self.status_is_error = false;
                self.server_offer = None;
                Task::none()
            }
            Message::NavBack => self.navigate(false),
            Message::NavForward => self.navigate(true),
            Message::ToggleFold => {
                self.fold(crate::code_editor::FoldCommand::AtCursor);
                Task::none()
            }
            Message::FoldAll => {
                self.fold(crate::code_editor::FoldCommand::All(true));
                Task::none()
            }
            Message::UnfoldAll => {
                self.fold(crate::code_editor::FoldCommand::All(false));
                Task::none()
            }
            Message::FoldClick(line, scope) => {
                self.fold(crate::code_editor::FoldCommand::Line(line, scope));
                Task::none()
            }
            Message::EditorRightClick(at) => {
                let annotated = self.blame_shown();
                let origin = crate::editor::text_origin(
                    &self.config,
                    self.editor.content.line_count(),
                    annotated,
                );
                self.gutter_click = self.config.show_line_numbers && at.x < origin.x;
                self.gutter_line = self
                    .gutter_click
                    .then(|| self.editor.content.line_at_y(at.y - origin.y))
                    .flatten();
                // The context menu acts on the clicked line: move the caret
                // there, unless there is a selection to act on.
                if self.editor.content.selection().is_none() && !self.gutter_click {
                    self.editor
                        .content
                        .perform(text_editor::Action::Click(at - origin));
                    self.undo_new_group = true;
                }
                Task::none()
            }
            Message::ChangeBar(line) => {
                let Some(path) = self.current_file.clone() else {
                    return Task::none();
                };
                let same = self
                    .revert
                    .as_ref()
                    .is_some_and(|(p, l, r)| *p == path && *l == line && *r == self.buffer_stamp());
                self.revert = (!same).then(|| (path, line, self.buffer_stamp()));
                Task::none()
            }
            Message::CloseRevert => {
                self.revert = None;
                Task::none()
            }
            Message::RevertChange => {
                let Some((path, line, _)) = self.revert.take() else {
                    return Task::none();
                };
                if self.editing_blocked() || self.current_file.as_ref() != Some(&path) {
                    return Task::none();
                }
                let base = self.marks.as_ref().and_then(|m| m.base.clone());
                let old = crate::editor::buffer_text(&self.editor.content);
                match crate::diff::marks::revert(base.as_deref(), &old, line) {
                    Some(new_text) => self.apply_line_edit(&old, new_text, 0, None),
                    None => Task::none(),
                }
            }
            Message::GotoDefinition => self.goto_definition(),
            Message::GotoTypeDefinition => self.goto_by_server(false),
            Message::OrganizeImports => {
                if self.is_formatting {
                    return Task::none();
                }
                let (Some(path), Some(ext)) =
                    (self.current_file.clone(), self.editor.language.clone())
                else {
                    return Task::none();
                };
                if is_untitled(&path) || crate::lsp::server_of(&ext).is_none() {
                    self.set_status(t!("outline.no_server"), true);
                    return Task::none();
                }
                self.is_formatting = true;
                self.set_status(t!("status.formatting"), false);
                let content = crate::editor::buffer_text(&self.editor.content);
                let job = FormatJob {
                    path: Some(path.clone()),
                    source: fingerprint(&content),
                };
                Task::perform(
                    async move { crate::lsp::organize_imports(&ext, &path, &content).await },
                    move |result| Message::FileFormatted(job.clone(), result),
                )
            }
            Message::GotoImplementation => self.goto_by_server(true),
            Message::OpenReference(index) => {
                let ActiveOverlay::References(list) = &self.overlay else {
                    return Task::none();
                };
                let Some(hit) = list.items.get(index).map(|r| r.hit.clone()) else {
                    return Task::none();
                };
                self.overlay = ActiveOverlay::None;
                self.jump_to(hit)
            }
            Message::InstallServer(binary) => {
                if !self.servers.installing.insert(binary) {
                    return Task::none();
                }
                self.set_status(t!("status.server_installing", name = binary), false);
                Task::perform(crate::lsp_install::install(binary), move |result| {
                    Message::ServerInstalled(binary, result)
                })
            }
            Message::ServerInstalled(binary, result) => {
                self.servers.installing.remove(binary);
                match result {
                    Ok(()) => {
                        self.servers.ready.insert(binary);
                        self.servers.removable.insert(binary);
                        self.set_status(t!("status.server_installed", name = binary), false);
                    }
                    Err(error) => {
                        self.set_status(
                            t!("status.server_install_failed", name = binary, error = error),
                            true,
                        );
                    }
                }
                Task::none()
            }
            Message::ServersChecked((ready, removable)) => {
                self.servers.ready = ready;
                self.servers.removable = removable;
                Task::none()
            }
            Message::UninstallServer(binary) => {
                if !self.servers.removing.insert(binary) {
                    return Task::none();
                }
                Task::perform(crate::lsp_install::uninstall(binary), move |result| {
                    Message::ServerUninstalled(binary, result)
                })
            }
            Message::ServerUninstalled(binary, result) => {
                self.servers.removing.remove(binary);
                match result {
                    Ok(()) => {
                        self.servers.ready.remove(binary);
                        self.servers.removable.remove(binary);
                        self.set_status(t!("status.server_uninstalled", name = binary), false);
                    }
                    Err(error) => {
                        let (name, why) = (binary, error);
                        let m = t!("status.server_uninstall_failed", name = name, error = why);
                        self.set_status(m, true);
                    }
                }
                Task::none()
            }
            Message::DefinitionFound(word, outcome) => {
                use crate::lsp::Outcome;
                // Closed (or replaced) while searching: the answer is dropped.
                let waiting = matches!(
                    &self.overlay,
                    ActiveOverlay::References(r) if r.state == RefState::Searching && r.word == word
                );
                if !waiting {
                    return Task::none();
                }
                match outcome {
                    Outcome::Found(hit) => {
                        self.overlay = ActiveOverlay::None;
                        self.jump_to(hit)
                    }
                    Outcome::References(items) => {
                        self.overlay = ActiveOverlay::References(RefList {
                            word,
                            items,
                            selected: 0,
                            state: RefState::Ready,
                        });
                        Task::none()
                    }
                    other => {
                        let message = match other {
                            Outcome::NotFound => t!("status.no_definition", name = word),
                            Outcome::AtDefinition => t!("status.no_references", name = word),
                            Outcome::Failed(error) => {
                                t!("status.language_server_error", error = error)
                            }
                            // Answered by the pattern search instead.
                            Outcome::Found(_)
                            | Outcome::References(_)
                            | Outcome::Unsupported(_)
                            | Outcome::NoServer(..) => {
                                return Task::none();
                            }
                        };
                        self.overlay = ActiveOverlay::References(RefList {
                            word,
                            items: Vec::new(),
                            selected: 0,
                            state: RefState::Empty(message.to_string()),
                        });
                        Task::none()
                    }
                }
            }
            Message::NextBookmark => self.jump_to_bookmark(true),
            Message::PrevBookmark => self.jump_to_bookmark(false),
            Message::ClearBookmarks => {
                if let Some(path) = self.current_file.clone() {
                    self.bookmarks.clear(&path);
                    self.set_status(t!("status.bookmarks_cleared"), false);
                }
                Task::none()
            }
            Message::IndentSelection => {
                if self.editing_blocked() {
                    return Task::none();
                }
                let Some(span) = self.selection_span() else {
                    return Task::none();
                };
                let (first, last) = span.lines();
                let text = crate::editor::buffer_text(&self.editor.content);
                let new_text = text_ops::indent_lines(&text, first, last, &self.indent_unit());
                self.apply_line_edit(&text, new_text, 0, Some(span))
            }
            Message::DedentLine => {
                if self.editing_blocked() {
                    return Task::none();
                }
                let text = crate::editor::buffer_text(&self.editor.content);
                let span = self.selection_span();
                let (first, last) = self.lines_to_edit(span);
                let new_text = text_ops::dedent_lines(
                    &text,
                    first,
                    last,
                    &self.indent_unit(),
                    self.indent().1,
                );
                self.apply_line_edit(&text, new_text, 0, span)
            }
            Message::OpenRecentFile(path) => {
                if !path.exists() {
                    self.config
                        .recent_files
                        .retain(|f| *f != path.to_string_lossy().as_ref());
                    self.config.save();
                    self.set_status(
                        format!("{}: {}", t!("status.error"), t!("status.no_file")),
                        true,
                    );
                    return Task::none();
                }
                // The tab appears once the file has been read.
                if self.sidebar.files().contains(&path) {
                    self.activate(path)
                } else {
                    Task::perform(read_file(path, false), Message::FileOpened)
                }
            }
            Message::ShortcutChecked(inner, focused) => {
                if focused {
                    Task::none()
                } else {
                    self.handle(*inner)
                }
            }
            Message::SearchEnter { shift } => self.handle(Message::Search(if shift {
                SearchMessage::FindPrevious
            } else {
                SearchMessage::Find
            })),
            Message::SearchEnterChecked {
                shift,
                in_find_field,
            } => {
                if in_find_field {
                    self.handle(Message::SearchEnter { shift })
                } else {
                    Task::none()
                }
            }
            Message::EscapePressed => {
                // A dialog of a plugin is over everything but the overlays
                // of the editor: it goes first.
                if self.overlay == ActiveOverlay::None {
                    if let Some(key) = self.plugin_front_dialog().cloned() {
                        self.plugin_close_dialog(&key);
                        return Task::none();
                    }
                }
                // Several cursors go back to one.
                if self.overlay == ActiveOverlay::None && self.editor.content.cursor_count() > 1 {
                    self.editor.content.clear_extras();
                    return Task::none();
                }
                // Innermost first: the list of completions, a pending
                // rebind, a dialog, the Preferences window, the search card.
                self.completion.snippet = None;
                if self.overlay == ActiveOverlay::FindInFiles
                    && (self.finder.menu.is_some() || self.finder.confirm_all)
                {
                    self.finder.menu = None;
                    self.finder.confirm_all = false;
                } else if self.overlay == ActiveOverlay::Everywhere
                    && (self.everywhere.abbreviating.is_some() || self.everywhere.filter_open)
                {
                    self.everywhere.abbreviating = None;
                    self.everywhere.filter_open = false;
                } else if self.completion_visible() {
                    self.completion_close();
                } else if self.completion.hint.is_some() {
                    self.completion.hint = None;
                } else if self.capturing.is_some() {
                    self.capturing = None;
                } else if !matches!(self.overlay, ActiveOverlay::None) {
                    if self.overlay_is_dismissable() {
                        self.overlay = ActiveOverlay::None;
                    }
                } else if self.show_preferences {
                    self.show_preferences = false;
                } else if self.show_search {
                    self.show_search = false;
                }
                Task::none()
            }
            Message::CheckForUpdate => {
                self.overlay = ActiveOverlay::UpdateChecking;
                // The plugins are updated with the app.
                self.plugins.update_all = true;
                let plugins = self.plugin_update(crate::plugins::PluginMsg::CheckUpdates);
                Task::batch([
                    Task::perform(crate::updater::check_for_update(), Message::UpdateChecked),
                    plugins,
                ])
            }
            Message::OpenChangelog => {
                self.overlay = ActiveOverlay::Changelog;
                Task::none()
            }
            Message::UpdateChecked(Ok(Some((version, notes)))) => {
                self.overlay = ActiveOverlay::UpdateAvailable(version, notes);
                Task::none()
            }
            Message::UpdateChecked(Ok(None)) => {
                self.overlay = ActiveOverlay::UpdateUpToDate;
                Task::none()
            }
            Message::UpdateChecked(Err(e)) => {
                self.overlay = ActiveOverlay::UpdateError(e);
                Task::none()
            }
            Message::ConfirmInstallUpdate(version) => {
                self.overlay = ActiveOverlay::UpdateDownloading;
                Task::perform(
                    crate::updater::download_update(version),
                    Message::UpdateDownloaded,
                )
            }
            Message::UpdateDownloaded(Ok((path, hash))) => {
                self.overlay = ActiveOverlay::UpdateInstalling;
                Task::perform(
                    crate::updater::install_update(path, hash),
                    Message::UpdateInstalled,
                )
            }
            Message::UpdateDownloaded(Err(e)) => {
                self.overlay = ActiveOverlay::UpdateError(e);
                Task::none()
            }
            Message::UpdateInstalled(Ok(())) => {
                if let Err(e) = self.save_session_now() {
                    self.overlay = ActiveOverlay::UpdateError(e);
                    return Task::none();
                }
                self.config.save();
                // On success this never returns — the process re-execs and exits.
                if let Err(e) = crate::updater::restart() {
                    self.overlay = ActiveOverlay::UpdateError(e);
                }
                Task::none()
            }
            Message::UpdateInstalled(Err(e)) => {
                self.overlay = ActiveOverlay::UpdateError(e);
                Task::none()
            }
            Message::SetViewMode(mode) => {
                self.view_mode = mode;
                Task::none()
            }
            Message::JsonToggle(path) => {
                // Ctrl+click opens every node, Alt+click closes every node.
                if self.mod_ctrl {
                    self.handle(Message::JsonExpandAll)
                } else if self.mod_alt {
                    self.handle(Message::JsonCollapseAll)
                } else {
                    let collapsed = std::sync::Arc::make_mut(&mut self.json_collapsed);
                    if !collapsed.remove(&path) {
                        collapsed.insert(path);
                    }
                    self.preview_rev += 1;
                    Task::none()
                }
            }
            Message::JsonExpandAll => {
                self.json_collapsed = Default::default();
                self.preview_rev += 1;
                Task::none()
            }
            Message::JsonCollapseAll => {
                if let PreviewDoc::Json(value) = &self.preview {
                    self.json_collapsed =
                        std::sync::Arc::new(crate::preview::json::container_paths(value));
                    self.preview_rev += 1;
                }
                Task::none()
            }
            Message::ModifiersChanged { ctrl, alt, shift } => {
                // Ctrl let go: the tab reached is the one seen last.
                if !ctrl {
                    self.tab_switch = None;
                }
                self.mod_ctrl = ctrl;
                self.mod_alt = alt;
                self.mod_shift = shift;
                Task::none()
            }
            Message::MarkdownLinkClicked(href) => {
                crate::preview::markdown::find_offset_command(&href).map(Message::PreviewScrollTo)
            }
            Message::PreviewScrollTo(Some(y)) => iced::widget::operation::scroll_to(
                crate::preview::markdown::scrollable_id(),
                iced::widget::scrollable::AbsoluteOffset {
                    x: 0.0,
                    // A little breathing room above the heading, clamped so
                    // targets near the top don't ask for a negative scroll.
                    y: (y - 12.0).max(0.0),
                },
            ),
            Message::PreviewScrollTo(None) => Task::none(),
        }
    }

    /// Asks the renderer where the open menu's button sits, so its dropdown
    /// can be anchored under it regardless of label/font/DPI.
    fn query_menu_bounds(&self) -> Task<Message> {
        match self.open_menu {
            Some(m) => iced::widget::selector::find(iced::widget::selector::id(
                crate::menu_bar::menu_id(m),
            ))
            .map(move |target| Message::MenuBounds(m, target.and_then(|t| t.visible_bounds()))),
            None => Task::none(),
        }
    }

    /// Whether the open overlay can be closed by hand.
    fn overlay_is_dismissable(&self) -> bool {
        !matches!(
            self.overlay,
            ActiveOverlay::UpdateDownloading | ActiveOverlay::UpdateInstalling
        )
    }

    /// Recomputes the selection label of the status bar.
    fn refresh_selection_info(&mut self) {
        let key = (
            self.editor.content.selection_range(),
            self.editor.content.revision(),
        );
        if self.selection_key == Some(key) {
            return;
        }
        self.selection_key = Some(key);
        self.selection_info = self
            .editor
            .content
            .selection_counts()
            .map(|(chars, lines)| {
                if lines > 1 {
                    t!("status.selection_lines", lines = lines, chars = chars).to_string()
                } else {
                    t!("status.selection_chars", chars = chars).to_string()
                }
            });
    }

    /// Shows a message in the status bar (in red for an error).
    fn set_status(&mut self, message: impl ToString, is_error: bool) {
        self.status_message = message.to_string();
        self.status_is_error = is_error;
        self.server_offer = None;
    }

    /// After a file opens: offers to install the language server it could
    /// use, if there is one to install and none is there yet.
    fn offer_language_server(&mut self, path: &Path) {
        let Some(extension) = path.extension().and_then(|e| e.to_str()) else {
            return;
        };
        self.server_offer = crate::lsp_install::offer_for(&extension.to_lowercase())
            .filter(|info| !self.servers.installing.contains(info.binary));
    }

    /// Whether a panel or dialog owns the keyboard, or there is nothing to
    /// edit, so editing commands (Tab, line operations) must leave the buffer
    /// alone. A hidden editor (Preview mode) counts: the user can't see what
    /// they would change.
    /// The large file in front, with the part it shows.
    fn large_view(&self) -> Option<LargeView> {
        self.current_file
            .as_ref()
            .and_then(|p| self.large.get(p).copied())
    }

    fn editing_blocked(&self) -> bool {
        self.editing_blocked_by(!matches!(self.overlay, ActiveOverlay::None))
    }

    /// `covered`: something lies over the editor.
    fn editing_blocked_by(&self, covered: bool) -> bool {
        covered
            || self.current_file.is_none()
            || self.large_view().is_some()
            || self.show_preferences
            || (self.view_mode == ViewMode::Preview && self.preview_kind().is_some())
    }

    /// Runs `m` unless a search field has the keyboard (it owns Tab, Ctrl+A…).
    fn check_search_focus_then(&mut self, m: Message) -> Task<Message> {
        if self.show_search && m.is_editor_shortcut() {
            iced::advanced::widget::operate(crate::search::search_field_focused())
                .map(move |focused| Message::ShortcutChecked(Box::new(m.clone()), focused))
        } else {
            self.handle(m)
        }
    }

    /// How the file in front is indented: (spaces, columns of a level).
    fn indent(&self) -> (bool, usize) {
        self.current_file
            .as_ref()
            .and_then(|p| self.indents.get(p).copied())
            .unwrap_or((self.config.use_spaces, self.config.tab_width))
    }

    /// One level of indentation, per the tab settings.
    fn indent_unit(&self) -> String {
        let (spaces, width) = self.indent();
        if spaces {
            " ".repeat(width)
        } else {
            "\t".to_string()
        }
    }

    /// What the Tab key types: spaces up to the next tab stop, or a tab.
    fn tab_text(&self) -> String {
        let (spaces, width) = self.indent();
        if !spaces {
            return "\t".to_string();
        }
        let (line, col) = self.editor.content.cursor_position();
        let before = self
            .editor
            .content
            .line(line)
            .map(|l| l[..floor_boundary(&l, col)].to_string())
            .unwrap_or_default();
        " ".repeat(text_ops::spaces_to_tab_stop(&before, width))
    }

    /// Whether `edit` would change the buffer: Backspace at the very start
    /// or Delete at the very end do nothing, and shouldn't leave an undo step.
    fn edit_changes_text(&self, edit: &text_editor::Edit) -> bool {
        if self.editor.content.selection().is_some() {
            return true;
        }
        let (line, col) = self.editor.content.cursor_position();
        match edit {
            text_editor::Edit::Backspace => line > 0 || col > 0,
            text_editor::Edit::Delete => {
                let last = self.editor.content.line_count().saturating_sub(1);
                line < last
                    || self
                        .editor
                        .content
                        .line(line)
                        .is_some_and(|l| col < l.len())
            }
            _ => true,
        }
    }

    /// An action in the text area: undo grouping, auto-indent, auto-pairing,
    /// then the action itself.
    fn editor_action(&mut self, action: text_editor::Action) -> Task<Message> {
        let is_edit = action.is_edit();

        // With several cursors the edit is done at each (Enter keeps the
        // indentation, the rest is plain).
        if self.editor.content.cursor_count() > 1
            && matches!(
                action,
                text_editor::Action::Edit(_)
                    | text_editor::Action::Move(_)
                    | text_editor::Action::Select(_)
            )
        {
            if is_edit {
                self.push_undo();
                self.undo_new_group = true;
                self.editor
                    .content
                    .perform_multi(action, self.config.auto_indent);
                self.is_dirty = true;
            } else {
                self.editor.content.perform_multi(action, false);
            }
            return Task::none();
        }

        if is_edit {
            if let text_editor::Action::Edit(edit) = &action {
                if !self.edit_changes_text(edit) {
                    return Task::none();
                }
                match edit {
                    text_editor::Edit::Insert(ch) => {
                        // Typing right after deletions is a step of its own.
                        let after_deletion =
                            self.undo_deletion.take().is_some() && !self.undo_new_group;
                        if self.editor.content.selection().is_some() {
                            // Typing over a selection is a step of its own.
                            self.push_undo();
                            self.undo_new_group = ch.is_whitespace();
                        } else if ch.is_whitespace() {
                            if after_deletion {
                                self.push_undo();
                            }
                            // Whitespace ends the current word group; next non-ws starts a new one
                            self.undo_new_group = true;
                        } else if self.undo_new_group || after_deletion {
                            // First non-whitespace of a new group: snapshot now
                            self.push_undo();
                            self.undo_new_group = false;
                        }
                        // else: mid-word, stay in current group
                    }
                    text_editor::Edit::Backspace | text_editor::Edit::Delete
                        if self.editor.content.selection().is_none() =>
                    {
                        // Backspaces in a row (or Deletes in a row) are one
                        // step, as the letters of a typed word are; a move
                        // or another edit in between opens a new one.
                        let kind = std::mem::discriminant(edit);
                        if self.undo_new_group || self.undo_deletion != Some(kind) {
                            self.push_undo();
                            self.undo_new_group = false;
                            self.undo_deletion = Some(kind);
                        }
                    }
                    _ => {
                        // Enter, Paste, a deleted selection: always their own undo point
                        self.push_undo();
                        self.undo_new_group = true;
                        self.undo_deletion = None;
                    }
                }
                if let text_editor::Edit::Paste(text) = edit {
                    if text.len() > MAX_PASTE_BYTES {
                        self.undo_stack.pop_back();
                        self.set_status(
                            format!(
                                "{}: {}",
                                t!("status.error"),
                                t!("status.err_paste_too_large")
                            ),
                            true,
                        );
                        return Task::none();
                    }
                }
            }
        } else {
            // Motion action: next edit opens a new group
            self.undo_new_group = true;
            self.pending_closers.clear();
        }

        // Auto-indent: replicate leading whitespace of current line on Enter
        if self.config.auto_indent {
            if let text_editor::Action::Edit(text_editor::Edit::Enter) = &action {
                self.pending_closers.clear();
                let (line_idx, col) = self.editor.content.cursor_position();
                // Only the whitespace before the cursor: pressing Enter
                // inside the indentation must not duplicate all of it.
                let indent: String = self
                    .editor
                    .content
                    .line(line_idx)
                    .map(|l| {
                        l.chars()
                            .take_while(|c| *c == ' ' || *c == '\t')
                            .collect::<String>()
                    })
                    .unwrap_or_default();
                let indent = indent[..indent.len().min(col)].to_string();
                // Deeper after `{`, `(`, `[` (and `:` in Python); a pair
                // `{|}` is split. Not over a selection, which Enter replaces.
                let (before, after) = match self.editor.content.line(line_idx) {
                    Some(l) if self.editor.content.selection().is_none() => {
                        let col = col.min(l.len());
                        (
                            l.get(..col).unwrap_or_default().to_string(),
                            l.get(col..).unwrap_or_default().to_string(),
                        )
                    }
                    _ => Default::default(),
                };
                let unit = self.indent_unit();
                let colon = self.editor.language.as_deref() == Some("py");
                let (inner, closer) =
                    crate::code_editor::smart_enter(&before, &after, &indent, &unit, colon);
                self.editor
                    .content
                    .perform(text_editor::Action::Edit(text_editor::Edit::Enter));
                if !inner.is_empty() {
                    self.editor.content.perform(text_editor::Action::Edit(
                        text_editor::Edit::Paste(std::sync::Arc::new(inner.clone())),
                    ));
                }
                // `{|}`: the closer goes down to a line of its own.
                if let Some(closer) = closer {
                    self.editor
                        .content
                        .perform(text_editor::Action::Edit(text_editor::Edit::Enter));
                    if !closer.is_empty() {
                        self.editor.content.perform(text_editor::Action::Edit(
                            text_editor::Edit::Paste(std::sync::Arc::new(closer)),
                        ));
                    }
                    self.editor
                        .content
                        .select_bytes(line_idx + 1, inner.len(), 0);
                }
                self.is_dirty = true;
                return Task::none();
            }
        }

        // Backspace right after an opener the auto-pairing closed takes the
        // closer with it.
        if matches!(
            action,
            text_editor::Action::Edit(text_editor::Edit::Backspace)
        ) && self.editor.content.selection().is_none()
            && self.backspace_pair()
        {
            self.is_dirty = true;
            return Task::none();
        }
        // Typing a closer on a line of its own steps back one indentation.
        if let text_editor::Action::Edit(text_editor::Edit::Insert(ch @ ('}' | ')' | ']'))) =
            &action
        {
            if self.config.auto_indent {
                self.dedent_before_closer(*ch);
            }
        }

        // Auto-pairing of brackets and quotes
        if let text_editor::Action::Edit(text_editor::Edit::Insert(ch)) = &action {
            if let Some(command) = self.autopair(*ch, &action) {
                return command;
            }
            self.shift_pending_closers(ch.len_utf8());
        } else if is_edit {
            self.pending_closers.clear();
        }

        self.editor.content.perform(action);
        if is_edit {
            self.is_dirty = true;
        }
        Task::none()
    }

    /// Backspace between an opener and the closer that was added with it:
    /// both go. `false` when the cursor is not in such a pair.
    fn backspace_pair(&mut self) -> bool {
        use text_editor::{Action, Edit};
        let (line, col) = self.editor.content.cursor_position();
        let Some(i) = self
            .pending_closers
            .iter()
            .position(|&(l, c, _)| l == line && c == col)
        else {
            return false;
        };
        let closer = self.pending_closers[i].2;
        let opener = match closer {
            ')' => '(',
            ']' => '[',
            '}' => '{',
            other => other,
        };
        let Some(text) = self.editor.content.line(line).map(|l| l.to_string()) else {
            return false;
        };
        let col = col.min(text.len());
        if text.get(..col).and_then(|b| b.chars().next_back()) != Some(opener)
            || text.get(col..).and_then(|a| a.chars().next()) != Some(closer)
        {
            return false;
        }
        self.pending_closers.remove(i);
        self.editor.content.perform(Action::Edit(Edit::Delete));
        self.editor.content.perform(Action::Edit(Edit::Backspace));
        true
    }

    /// A closer typed after nothing but indentation takes one level off it.
    fn dedent_before_closer(&mut self, closer: char) {
        let (line, col) = self.editor.content.cursor_position();
        if self.editor.content.selection().is_some() {
            return;
        }
        let Some(text) = self.editor.content.line(line).map(|l| l.to_string()) else {
            return;
        };
        let col = col.min(text.len());
        let (before, after) = (&text[..col], &text[col..]);
        if before.is_empty()
            || !before.chars().all(|c| c == ' ' || c == '\t')
            || after.starts_with(closer)
        {
            return;
        }
        let unit = self.indent_unit();
        let remove = if before.ends_with(&unit) {
            unit.chars().count()
        } else if before.ends_with('\t') {
            1
        } else {
            // Less than a unit of spaces: all of them.
            before.chars().count().min(unit.chars().count())
        };
        for _ in 0..remove {
            self.editor
                .content
                .perform(text_editor::Action::Edit(text_editor::Edit::Backspace));
        }
    }

    /// Characters typed on the cursor's line push the auto-inserted closers
    /// after it to the right.
    fn shift_pending_closers(&mut self, bytes: usize) {
        let (line, col) = self.editor.content.cursor_position();
        for (l, c, _) in &mut self.pending_closers {
            if *l == line && *c >= col {
                *c += bytes;
            }
        }
    }

    /// (line count, topmost line an edit may touch), taken before an edit.
    fn edit_probe(&self) -> (usize, usize) {
        let (line, _) = self.editor.content.cursor_position();
        let above = self
            .editor
            .content
            .selection()
            .map_or(0, |s| s.matches('\n').count());
        (self.editor.content.line_count(), line.saturating_sub(above))
    }

    fn fold(&mut self, command: crate::code_editor::FoldCommand) {
        let kind = crate::folding::Kind::of(self.editor.language.as_deref());
        self.editor.content.fold(kind, command);
    }

    /// Moves the current file's bookmarks after an edit that changed the line
    /// count (see `edit_probe`).
    fn shift_bookmarks(&mut self, count_before: usize, top_before: usize) {
        let (Some(path), count) = (&self.current_file, self.editor.content.line_count()) else {
            return;
        };
        if count == count_before {
            return;
        }
        let top = top_before.min(self.editor.content.cursor_position().0);
        let delta = count as isize - count_before as isize;
        self.bookmarks.shift(path, top, delta);
        self.shift_problems(top, delta);
    }

    /// Runs an edit of the buffer and moves the bookmarks along with it.
    fn edited(&mut self, edit: impl FnOnce(&mut Self)) {
        let (count, top) = self.edit_probe();
        edit(self);
        self.shift_bookmarks(count, top);
    }

    fn bookmark_lines(&self) -> &[usize] {
        self.current_file
            .as_deref()
            .map_or(&[], |path| self.bookmarks.lines(path))
    }

    /// What the menus need to know of the editor to switch entries on or off.
    fn menu_editor(&self) -> crate::menus::EditorState {
        let kind = crate::folding::Kind::of(self.editor.language.as_deref());
        let has_text = self.editor.content.has_text();
        // The lines a line command would touch: can they go up, or down?
        let (first, last) = self.lines_to_edit(self.selection_span());
        let first_line = first == 0;
        let last_line = last + 1 >= self.editor.content.line_count();
        crate::menus::EditorState {
            can_undo: !self.undo_stack.is_empty(),
            can_redo: !self.redo_stack.is_empty(),
            can_back: !self.nav_back.is_empty(),
            can_forward: !self.nav_forward.is_empty(),
            can_reopen: !self.closed_tabs.is_empty(),
            several_tabs: self.sidebar.files().len() > 1,
            can_check: self.config.lint_enabled && self.lint_can_check(),
            has_problems: !self.visible_problems().is_empty(),
            fold: self.editor.content.fold_state(kind),
            has_file: self.current_file.is_some(),
            can_edit: !self.editing_blocked(),
            can_comment: comment_style(self.editor.language.as_deref()).is_some(),
            first_line,
            last_line,
            has_selection: self.editor.content.selection_range().is_some(),
            any_modified: !self.dirty_files().is_empty(),
            has_tabs: !self.sidebar.files().is_empty(),
            can_split: self.current_file.as_deref().is_some_and(|p| !is_virtual(p)),
            can_complete: self.config.completion_enabled
                && (self.plugins.editor_focus.is_some() || has_text),
            can_hint: self.config.completion_enabled && has_text && self.has_language_server(),
        }
    }

    fn bookmark_state(&self) -> crate::menu_bar::BookmarkState {
        let line = self.editor.content.cursor_position().0;
        crate::menu_bar::BookmarkState {
            on_line: self
                .current_file
                .as_deref()
                .is_some_and(|path| self.bookmarks.has(path, line)),
            any: !self.bookmark_lines().is_empty(),
        }
    }

    /// Asks the language server where the name under the cursor is defined.
    fn goto_definition(&mut self) -> Task<Message> {
        let Some(path) = self.current_file.clone() else {
            return Task::none();
        };
        let (line, col) = self.editor.content.cursor_position();
        let Some(word) = self
            .editor
            .content
            .line(line)
            .and_then(|text| crate::definition::word_at(&text, col).map(str::to_string))
        else {
            return Task::none();
        };
        let language = self
            .editor
            .language
            .clone()
            .or_else(|| path.extension().map(|e| e.to_string_lossy().into_owned()));
        // Without a known language only the pattern search can answer.
        let language = language.unwrap_or_default();
        let text = crate::editor::buffer_text(&self.editor.content);
        // Open files: the unsaved text of the modified ones.
        let open: Vec<_> = self
            .sidebar
            .files()
            .iter()
            .filter(|p| **p != path && !is_untitled(p))
            .map(|p| {
                let text = self
                    .file_cache
                    .get(p)
                    .filter(|c| c.dirty)
                    .and_then(|c| c.content.as_ref())
                    .map(|c| c.text());
                (p.clone(), text)
            })
            .collect();
        self.overlay = ActiveOverlay::References(RefList::searching(word.clone()));
        let name = word.clone();
        Task::perform(
            async move {
                // A language server when there is one, the pattern search
                // otherwise.
                match crate::lsp::definition(&language, &path, &text, line, col).await {
                    crate::lsp::Outcome::Unsupported(_) | crate::lsp::Outcome::NoServer(..) => {
                        patterns(word, path, text, line, col, open).await
                    }
                    // On the definition itself: where it is used.
                    crate::lsp::Outcome::AtDefinition => {
                        match crate::lsp::references(&language, &path, &text, line, col).await {
                            // Not used anywhere: said as such.
                            crate::lsp::Outcome::NotFound => crate::lsp::Outcome::AtDefinition,
                            other => other,
                        }
                    }
                    outcome => outcome,
                }
            },
            move |outcome| Message::DefinitionFound(name.clone(), outcome),
        )
    }

    /// Asks the language server for the type of the name under the cursor
    /// (`implementations` false) or for what implements it. There is no
    /// pattern search to fall back on.
    fn goto_by_server(&mut self, implementations: bool) -> Task<Message> {
        let (Some(path), Some(language)) =
            (self.current_file.clone(), self.editor.language.clone())
        else {
            return Task::none();
        };
        if is_untitled(&path) || crate::lsp::server_of(&language).is_none() {
            self.set_status(t!("outline.no_server"), true);
            return Task::none();
        }
        let (line, col) = self.editor.content.cursor_position();
        let Some(word) = self
            .editor
            .content
            .line(line)
            .and_then(|text| crate::definition::word_at(&text, col).map(str::to_string))
        else {
            return Task::none();
        };
        let text = crate::editor::buffer_text(&self.editor.content);
        self.overlay = ActiveOverlay::References(RefList::searching(word.clone()));
        Task::perform(
            async move {
                if implementations {
                    crate::lsp::implementation(&language, &path, &text, line, col).await
                } else {
                    crate::lsp::type_definition(&language, &path, &text, line, col).await
                }
            },
            move |outcome| Message::DefinitionFound(word.clone(), outcome),
        )
    }

    /// Goes to a definition, opening its file if need be.
    fn jump_to(&mut self, hit: crate::definition::Hit) -> Task<Message> {
        self.remember_place();
        self.go_to(hit)
    }

    /// The cursor's place, to come back to with Back.
    fn remember_place(&mut self) {
        let Some(path) = self.current_file.clone() else {
            return;
        };
        let (line, col) = self.editor.content.cursor_position();
        let place = Place { path, line, col };
        if self.nav_back.last() != Some(&place) {
            self.nav_back.push(place);
            if self.nav_back.len() > MAX_PLACES {
                self.nav_back.remove(0);
            }
        }
        self.nav_forward.clear();
    }

    /// Back (or forward) to the previous place of the history.
    fn navigate(&mut self, forward: bool) -> Task<Message> {
        let Some(path) = self.current_file.clone() else {
            return Task::none();
        };
        let (line, col) = self.editor.content.cursor_position();
        let here = Place { path, line, col };
        let (from, to) = if forward {
            (&mut self.nav_forward, &mut self.nav_back)
        } else {
            (&mut self.nav_back, &mut self.nav_forward)
        };
        // A place identical to the current one is no move at all.
        let target = loop {
            match from.pop() {
                Some(place) if place == here => continue,
                other => break other,
            }
        };
        let Some(target) = target else {
            self.set_status(t!("status.no_more_places"), false);
            return Task::none();
        };
        to.push(here);
        self.go_to(crate::definition::Hit {
            path: target.path,
            line: target.line,
            col: target.col,
            len: 0,
        })
    }

    /// Puts the cursor at a place, opening its file if need be.
    fn go_to(&mut self, hit: crate::definition::Hit) -> Task<Message> {
        let len = hit.len;
        self.undo_new_group = true;
        if self.current_file.as_ref() == Some(&hit.path) {
            self.editor.content.select_bytes(hit.line, hit.col, len);
            return Task::none();
        }
        self.pending_jump = Some((hit.path.clone(), hit.line, hit.col, len));
        let task = self.activate(hit.path);
        // A tab already in memory is active by now.
        self.apply_pending_jump();
        task
    }

    /// Puts the cursor where `pending_jump` says, once its file is active.
    fn apply_pending_jump(&mut self) {
        if let Some((path, line, col, len)) = self.pending_jump.take() {
            if self.current_file.as_ref() == Some(&path) {
                self.editor.content.select_bytes(line, col, len);
            } else if self.pending_open.as_ref() == Some(&path) {
                self.pending_jump = Some((path, line, col, len));
            }
        }
    }

    /// Moves the cursor to the next (or previous) bookmark of the file.
    fn jump_to_bookmark(&mut self, forward: bool) -> Task<Message> {
        let Some(path) = self.current_file.clone() else {
            return Task::none();
        };
        let (line, _) = self.editor.content.cursor_position();
        let total = self.editor.content.line_count();
        let target = if forward {
            self.bookmarks.next(&path, line, total)
        } else {
            self.bookmarks.previous(&path, line, total)
        };
        match target {
            Some(target) => {
                self.remember_place();
                crate::search::goto_line_start(&mut self.editor.content, target);
                self.undo_new_group = true;
            }
            None => self.set_status(t!("status.no_bookmarks"), false),
        }
        Task::none()
    }

    fn snapshot(&self) -> Snapshot {
        Snapshot {
            text: crate::editor::buffer_text(&self.editor.content),
            cursor: self.editor.content.cursor_position(),
            extras: self.editor.content.extra_carets(),
        }
    }

    fn push_undo(&mut self) {
        // A step is a copy of the whole text: on a file of megabytes, the steps
        // that come within the same second are one, so that typing does not
        // copy it at every Enter and every pasted line.
        const BIG: usize = 2 * 1024 * 1024;
        let big = self.undo_stack.newest_len().is_some_and(|n| n > BIG);
        if big
            && self
                .last_undo_at
                .is_some_and(|t| t.elapsed() < std::time::Duration::from_secs(1))
        {
            self.redo_stack.clear();
            return;
        }
        self.last_undo_at = Some(std::time::Instant::now());
        let snapshot = self.snapshot();
        self.undo_stack.push_back(snapshot);
        trim_history(&mut self.undo_stack);
        self.redo_stack.clear();
    }

    /// Keeps the undo history of the background tabs within a global budget
    /// (the largest histories go first).
    fn enforce_undo_budget(&mut self) {
        let mut total: usize = self.undo_cache.values().map(history_bytes).sum();
        while total > UNDO_TOTAL_BYTES {
            let Some((largest, size)) = self
                .undo_cache
                .iter()
                .map(|(p, h)| (p.clone(), history_bytes(h)))
                .max_by_key(|(_, size)| *size)
            else {
                break;
            };
            self.undo_cache.remove(&largest);
            total -= size;
        }
    }

    /// Puts the cursor at (`line`, byte column `col`), clamped to the buffer.
    fn restore_cursor(&mut self, line: usize, col: usize) {
        self.editor.content.select_bytes(line, col, 0);
    }

    /// Replaces the buffer with a history snapshot and puts the cursor back.
    fn restore_snapshot(&mut self, snap: Snapshot) -> Task<Message> {
        let (line, col) = snap.cursor;
        // Back to the saved text means nothing left to save.
        self.is_dirty = self.saved_fp != Some(fingerprint(&snap.text));
        self.undo_new_group = true;
        self.replace_buffer_text(&snap.text);
        self.restore_cursor(line, col);
        // The other cursors come back too (a text that is not the one they
        // were made for would put them wrong, so they are checked).
        self.editor.content.set_extra_carets(&snap.extras);
        Task::none()
    }

    /// Applies a whole-buffer text transformation as one undo step and moves
    /// the cursor to (`line`, `col`), or selects `selection`. A no-op
    /// transformation changes nothing.
    fn apply_text_edit(
        &mut self,
        new_text: String,
        line: usize,
        col: usize,
        selection: Option<Span>,
    ) -> Task<Message> {
        let old = crate::editor::buffer_text(&self.editor.content);
        if new_text == old {
            return Task::none();
        }
        // One copy of the buffer serves the undo step, the bookmarks and the
        // minimal replacement.
        self.undo_stack.push_back(Snapshot {
            text: old.clone(),
            cursor: self.editor.content.cursor_position(),
            extras: self.editor.content.extra_carets(),
        });
        trim_history(&mut self.undo_stack);
        self.redo_stack.clear();
        self.is_dirty = true;
        self.undo_new_group = true;
        if let Some(path) = &self.current_file {
            self.bookmarks.remap(path, &old, &new_text);
        }
        crate::editor::replace_text(&mut self.editor.content, &old, &new_text);
        match selection {
            Some(span) => {
                let len = self.span_len(span);
                self.editor
                    .content
                    .select_bytes(span.start.0, span.start.1, len);
            }
            None => self.restore_cursor(line, col),
        }
        Task::none()
    }

    /// Like `apply_text_edit` for a line command: the cursor (and selection)
    /// keep their place on the text, `line_shift` lines further down.
    fn apply_line_edit(
        &mut self,
        old: &str,
        new_text: String,
        line_shift: isize,
        selection: Option<Span>,
    ) -> Task<Message> {
        let (line, col) = self.editor.content.cursor_position();
        let new_line = (line as isize + line_shift).max(0) as usize;
        let col = remap_col(nth_line(old, line), nth_line(&new_text, new_line), col);
        let selection = selection.map(|s| remap_span(old, &new_text, s, line_shift));
        self.apply_text_edit(new_text, new_line, col, selection)
    }

    fn replace_buffer_text(&mut self, new_text: &str) {
        if let Some(path) = &self.current_file {
            if !self.bookmarks.lines(path).is_empty() {
                let old = crate::editor::buffer_text(&self.editor.content);
                self.bookmarks.remap(path, &old, new_text);
            }
        }
        crate::editor::set_buffer_text(&mut self.editor.content, new_text);
    }

    /// The bytes covered by `span` in the buffer as it is now.
    fn span_len(&self, span: Span) -> usize {
        let content = &self.editor.content;
        let len_of = |i: usize| content.line(i).map_or(0, |l| l.len());
        if span.start.0 == span.end.0 {
            return span.end.1.saturating_sub(span.start.1);
        }
        let middle: usize = (span.start.0 + 1..span.end.0).map(|i| len_of(i) + 1).sum();
        len_of(span.start.0).saturating_sub(span.start.1) + 1 + middle + span.end.1
    }

    /// The span of a selection running over several lines, from its anchor
    /// and its cursor.
    fn selection_span(&self) -> Option<Span> {
        let (start, end) = self.editor.content.selection_range()?;
        (start.0 != end.0).then_some(Span { start, end })
    }

    /// The lines a line command applies to: the selected ones, else the
    /// cursor's.
    fn lines_to_edit(&self, span: Option<Span>) -> (usize, usize) {
        span.map_or_else(
            || {
                let line = self.editor.content.cursor_position().0;
                (line, line)
            },
            Span::lines,
        )
    }

    /// Alt+Up / Alt+Down: the selected lines (or the cursor's) one line up or down.
    fn move_lines(&mut self, up: bool) -> Task<Message> {
        if self.editing_blocked() {
            return Task::none();
        }
        let text = crate::editor::buffer_text(&self.editor.content);
        let span = self.selection_span();
        let (first, last) = self.lines_to_edit(span);
        let (new_text, new_first) = text_ops::move_lines(&text, first, last, up);
        let shift = new_first as isize - first as isize;
        self.apply_line_edit(&text, new_text, shift, span)
    }

    /// Auto-closing of brackets and quotes for the typed character `ch`.
    /// `None` means "insert it normally".
    fn autopair(&mut self, ch: char, action: &text_editor::Action) -> Option<Task<Message>> {
        use text_editor::{Action, Edit};

        let closer = match ch {
            '(' => Some(')'),
            '[' => Some(']'),
            '{' => Some('}'),
            _ => None,
        };
        let is_quote = matches!(ch, '"' | '\'' | '`');
        let is_closer = matches!(ch, ')' | ']' | '}');
        // A selection is surrounded rather than replaced, whether or not the
        // brackets and quotes complete themselves; the selection stays on the
        // text. (The undo step was opened by `editor_action`.)
        let language = self.editor.language.as_deref();
        let markdown = matches!(language, Some("md" | "markdown"));
        let markup = matches!(
            language,
            Some("html" | "htm" | "xml" | "xsl" | "xsd" | "svg" | "jsx" | "tsx" | "vue")
        );
        let wrap = match ch {
            '<' if markup => Some('>'),
            '*' | '_' | '~' if markdown => Some(ch),
            _ => closer.or(is_quote.then_some(ch)),
        };
        if let (Some(close), Some(selected), Some(range)) = (
            wrap,
            self.editor.content.selection(),
            self.editor.content.selection_range(),
        ) {
            if !selected.is_empty() && self.editor.content.cursor_count() == 1 {
                self.pending_closers.clear();
                self.editor
                    .content
                    .perform(text_editor::Action::Edit(text_editor::Edit::Paste(
                        std::sync::Arc::new(format!("{ch}{selected}{close}")),
                    )));
                let ((line, col), _) = range;
                self.editor
                    .content
                    .select_bytes(line, col + ch.len_utf8(), selected.len());
                self.is_dirty = true;
                return Some(Task::none());
            }
        }

        let enabled = if is_quote {
            self.config.autocomplete_quotes
        } else {
            self.config.autocomplete_brackets
        };
        if !enabled || !(closer.is_some() || is_quote || is_closer) {
            return None;
        }

        let (line, col) = self.editor.content.cursor_position();
        let (before, after) = self
            .editor
            .content
            .line(line)
            .map(|l| {
                let col = col.min(l.len());
                (
                    l.get(..col).and_then(|s| s.chars().next_back()),
                    l.get(col..).and_then(|s| s.chars().next()),
                )
            })
            .unwrap_or((None, None));
        let selection = self.editor.content.selection();

        // Typing the closing character that was inserted automatically steps
        // over it; one the user typed (or any other) is just inserted.
        let pending = self
            .pending_closers
            .iter()
            .position(|&(l, c, p)| l == line && c == col && p == ch);
        if (is_closer || is_quote) && selection.is_none() && after == Some(ch) {
            if let Some(i) = pending {
                self.pending_closers.remove(i);
                // Replace the character with itself rather than moving the
                // cursor: a motion over a line just typed on would panic.
                self.editor.content.perform(Action::Edit(Edit::Delete));
                self.editor.content.perform(Action::Edit(Edit::Insert(ch)));
                return Some(Task::none());
            }
        }
        if is_closer {
            return None;
        }
        let close = closer.unwrap_or(ch);

        // With a selection, surround it instead of replacing it. (The undo
        // step was opened by `editor_action`.)
        if let Some(selected) = selection {
            self.pending_closers.clear();
            self.editor
                .content
                .perform(Action::Edit(Edit::Paste(std::sync::Arc::new(format!(
                    "{ch}{selected}{close}"
                )))));
            self.is_dirty = true;
            return Some(Task::none());
        }

        // Only pair before whitespace / a closer / line end; quotes also
        // not after a word character (apostrophes: don't, it's), and a
        // single quote not at all in Rust, where it mostly starts a lifetime.
        let room = after.is_none_or(|c| {
            c.is_whitespace() || matches!(c, ')' | ']' | '}' | ',' | ';' | ':' | '.')
        });
        let word_before = before.is_some_and(|c| c.is_alphanumeric() || c == '\\');
        let rust_lifetime = ch == '\'' && self.editor.language.as_deref() == Some("rs");
        if !room || (is_quote && word_before) || rust_lifetime {
            return None;
        }

        self.editor.content.perform(action.clone());
        self.editor
            .content
            .perform(Action::Edit(Edit::Insert(close)));
        self.is_dirty = true;
        // Back over the closer, which then sits right after the cursor.
        let (line, end) = self.editor.content.cursor_position();
        let col = end - close.len_utf8();
        self.editor.content.select_bytes(line, col, 0);
        self.pending_closers.push((line, col, close));
        Some(Task::none())
    }

    /// Watches the project folders, and the folders of the open files that
    /// are in none of them.
    /// Reads the open folders that changed, off the interface thread (a
    /// build writing in the project, a disk on the network).
    fn reload_projects(&mut self) -> Task<Message> {
        if self.projects_reloading {
            return Task::none();
        }
        let Some(job) = self.projects.reload_job() else {
            return Task::none();
        };
        // The tests have no runtime to run a task: it is done on the spot.
        if cfg!(test) {
            let loaded = job.run();
            self.projects.reloaded(loaded);
            return Task::none();
        }
        self.projects_reloading = true;
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || job.run())
                    .await
                    .unwrap_or_default()
            },
            Message::ProjectsReloaded,
        )
    }

    /// Reads the file again as `encoding` says, off the interface thread (a
    /// big file, a slow disk). `revert`: what was typed is dropped (it stays
    /// one Ctrl+Z away).
    fn reload_later(
        &mut self,
        path: PathBuf,
        encoding: crate::tabs::Encoding,
        revert: bool,
    ) -> Task<Message> {
        let read = {
            let path = path.clone();
            move || -> Result<Loaded, String> {
                let stamp = DiskStamp::of(&path);
                let bytes = std::fs::read(&path).map_err(file_io::read_error)?;
                let raw = encoding
                    .decode(&bytes)
                    .ok_or_else(|| t!("status.err_decode", name = encoding.label()).to_string())?;
                let (text, mut format) = TextFormat::decode(&raw);
                format.encoding = encoding;
                Ok(Loaded {
                    path,
                    text,
                    format,
                    stamp,
                })
            }
        };
        // The tests have no runtime to run a task: it is read on the spot.
        if cfg!(test) {
            return self.reloaded(path, revert, read());
        }
        Task::perform(
            async move {
                tokio::task::spawn_blocking(read)
                    .await
                    .map_err(|e| e.to_string())?
            },
            move |result| Message::Reloaded(path.clone(), revert, result),
        )
    }

    /// The file read again: it replaces what the editor shows, if that is
    /// still the same file (and still unchanged, unless reverting).
    fn reloaded(
        &mut self,
        path: PathBuf,
        revert: bool,
        result: Result<Loaded, String>,
    ) -> Task<Message> {
        match result {
            Ok(loaded) => {
                if self.current_file.as_ref() != Some(&path) || (self.is_dirty && !revert) {
                    return Task::none();
                }
                if revert {
                    self.is_dirty = false;
                }
                self.apply_reload(loaded);
            }
            Err(e) => self.set_status(e, true),
        }
        Task::none()
    }

    fn sync_watch(&mut self) {
        // Not in the tests: they would watch the temporary folders of all.
        if cfg!(test) {
            return;
        }
        let roots: Vec<PathBuf> = self.projects.roots.clone();
        let mut single: Vec<PathBuf> = Vec::new();
        for file in self.sidebar.files() {
            if is_untitled(file) || is_virtual(file) {
                continue;
            }
            if let Some(dir) = file.parent() {
                if !single.iter().any(|d| d == dir) {
                    single.push(dir.to_path_buf());
                }
            }
        }
        self.watch.set(&roots, &single);
    }

    /// Whether the tab for `path` holds unsaved changes.
    fn is_path_dirty(&self, path: &Path) -> bool {
        // The tabs of plugins hold their work outside the text buffers.
        if self.plugins.tabs.get(path).is_some_and(|t| t.dirty) {
            return true;
        }
        if self.plugins.tabs.contains_key(path) {
            return false;
        }
        if self.current_file.as_deref() == Some(path) {
            self.is_dirty
        } else {
            self.file_cache.get(path).is_some_and(|e| e.dirty)
        }
    }

    /// Closes a tab, asking first if that would discard unsaved changes.
    fn request_close(&mut self, path: PathBuf) -> Task<Message> {
        if self.is_path_dirty(&path) {
            self.overlay = ActiveOverlay::ConfirmClose(path);
            Task::none()
        } else {
            self.close_path(path)
        }
    }

    /// The tick after every message: the marks of the current file follow
    /// the edits and the repository.
    /// What each row of the small list in front sends, in order (none: the
    /// window in front has no such list).
    fn overlay_choices(&self) -> Vec<Message> {
        use crate::tabs::{Encoding, LineEnding};
        match &self.overlay {
            ActiveOverlay::LanguagePicker => {
                let mut rows = vec![Message::SetLanguage(String::new())];
                rows.extend(
                    crate::editor::SUPPORTED_LANGUAGES
                        .iter()
                        .map(|(token, _)| Message::SetLanguage(token.to_string())),
                );
                rows
            }
            ActiveOverlay::TextOptions(TextOption::Encoding) => Encoding::ALL
                .into_iter()
                .map(Message::ReloadEncoding)
                .chain(Encoding::ALL.into_iter().map(Message::SetEncoding))
                .collect(),
            ActiveOverlay::TextOptions(TextOption::LineEnding) => vec![
                Message::SetLineEnding(LineEnding::Lf),
                Message::SetLineEnding(LineEnding::Crlf),
            ],
            ActiveOverlay::TextOptions(TextOption::Indent) => [2, 3, 4, 8]
                .into_iter()
                .map(|n| Message::SetIndent(true, n))
                .chain([2, 4, 8].into_iter().map(|n| Message::SetIndent(false, n)))
                .chain([
                    Message::DetectIndent,
                    Message::ConvertIndent(true),
                    Message::ConvertIndent(false),
                ])
                .collect(),
            ActiveOverlay::SpellLanguages => {
                let mut rows = vec![Message::SpellLanguage(String::new())];
                rows.extend(
                    self.spell
                        .choices
                        .iter()
                        .map(|c| Message::SpellLanguage(c.clone())),
                );
                rows
            }
            ActiveOverlay::Spelling(found) => found
                .list
                .iter()
                .map(|s| Message::SpellReplace(s.clone()))
                .chain([Message::SpellAdd, Message::SpellIgnore])
                .collect(),
            _ => Vec::new(),
        }
    }

    fn after_message(&mut self, edited: bool) -> Task<Message> {
        // Another file in front: what it is compared with.
        if self.current_file != self.marks_asked {
            self.refresh_marks_base();
            return Task::none();
        }
        let spelling = self.refresh_spelling();
        // A new window starts with its first row.
        let now = std::mem::discriminant(&self.overlay);
        if self.overlay_seen != Some(now) {
            self.overlay_seen = Some(now);
            self.overlay_pick = 0;
        }
        // Small files follow the typing, at most every 150 ms (a diff of the
        // whole text each time is a cost); the tick catches up after.
        if edited
            && self.editor.content.line_count() < 3000
            && (cfg!(test) || self.marks_at.elapsed() >= std::time::Duration::from_millis(150))
        {
            self.marks_at = std::time::Instant::now();
            self.refresh_marks_now();
        }
        spelling
    }

    /// What the current file is compared with, as a plugin said it (none:
    /// no marks).
    fn refresh_marks_base(&mut self) {
        self.marks_asked = self.current_file.clone();
        let base = self
            .current_file
            .clone()
            .filter(|p| !is_untitled(p))
            .and_then(|path| Some((self.plugins.bases.get(&path).cloned()?, path)));
        match base {
            Some((base, path)) => self.plugin_base_loaded(path, base),
            None => self.marks = None,
        }
    }

    fn marks_base_loaded(&mut self, path: PathBuf, base: Option<Option<String>>) {
        if self.current_file.as_ref() != Some(&path) {
            return;
        }
        match base {
            None => self.marks = None,
            Some(base) => {
                self.marks = Some(MarksState {
                    path,
                    base,
                    marks: Default::default(),
                    paint: Default::default(),
                    rev: (u64::MAX, u64::MAX),
                });
                self.refresh_marks_now();
            }
        }
    }

    /// Compares the buffer with the base again, if the text changed.
    fn refresh_marks_now(&mut self) {
        let rev = self.buffer_stamp();
        let Some(state) = self.marks.as_mut().filter(|m| m.rev != rev) else {
            return;
        };
        if self.current_file.as_ref() != Some(&state.path) {
            return;
        }
        let text = crate::editor::buffer_text(&self.editor.content);
        state.marks = crate::diff::marks::compute(state.base.as_deref(), &text);
        state.paint = [
            std::sync::Arc::new(state.marks.added.clone()),
            std::sync::Arc::new(state.marks.modified.clone()),
            std::sync::Arc::new(state.marks.deleted.clone()),
        ];
        state.rev = rev;
    }

    /// Which text the buffer holds: changes with every edit and when the text
    /// is replaced as a whole.
    /// The text of the buffer, kept for as long as the buffer is not changed.
    fn buffer_text_shared(&self) -> std::sync::Arc<String> {
        let stamp = self.buffer_stamp();
        let mut kept = self.text_shared.borrow_mut();
        match kept.as_ref() {
            Some((at, text)) if *at == stamp => text.clone(),
            _ => {
                let text = std::sync::Arc::new(crate::editor::buffer_text(&self.editor.content));
                *kept = Some((stamp, text.clone()));
                text
            }
        }
    }

    fn buffer_stamp(&self) -> (u64, u64) {
        (self.editor.content.id(), self.editor.content.revision())
    }

    /// The changed-line marks of the current file, for the editor.
    fn change_paint(&self) -> Option<crate::code_editor::ChangePaint> {
        let p = crate::theme::palette(self.config.dark_mode);
        let revert = self
            .revert
            .as_ref()
            .filter(|(path, _, rev)| {
                self.current_file.as_ref() == Some(path) && *rev == self.buffer_stamp()
            })
            .map(|(_, line, _)| *line);
        let state = self
            .marks
            .as_ref()
            .filter(|m| self.current_file.as_ref() == Some(&m.path))?;
        let m = &state.marks;
        if m.added.is_empty() && m.modified.is_empty() && m.deleted.is_empty() {
            return None;
        }
        let rgb = |r, g, b| iced::Color::from_rgb8(r, g, b);
        Some(crate::code_editor::ChangePaint {
            added: state.paint[0].clone(),
            modified: state.paint[1].clone(),
            deleted: state.paint[2].clone(),
            added_color: rgb(0x4c, 0xaf, 0x50),
            modified_color: rgb(0x4a, 0x9f, 0xe0),
            deleted_color: rgb(0x9a, 0x9f, 0xab),
            revert,
            popup_background: p.elevated,
            popup_border: p.border_strong,
            popup_text: p.text,
        })
    }

    /// Writes the modified files that have a place on disk, a moment after
    /// the last edit (never over a file that changed under us).
    /// Writes the tabs that hold their text aside and have unsaved changes
    /// (a file changed on the disk meanwhile is left alone).
    /// How a file is written: as it is stored, with the final line break the
    /// settings ask for.
    fn save_format(&self, format: TextFormat) -> TextFormat {
        TextFormat {
            final_newline: format.final_newline || self.config.final_newline_on_save,
            ..format
        }
    }

    /// Takes the blanks at the end of the lines off before a save, when the
    /// settings ask for it (not at an automatic save: the space just typed
    /// would vanish under the cursor).
    fn trim_before_save(&mut self) {
        if !self.config.trim_trailing_whitespace {
            return;
        }
        let old = crate::editor::buffer_text(&self.editor.content);
        let new = crate::text_ops::trim_trailing_whitespace(&old);
        if new != old {
            let (line, col) = self.editor.content.cursor_position();
            let _ = self.apply_text_edit(new, line, col, None);
        }
    }

    fn save_cached_tabs(&self) -> Vec<Task<Message>> {
        let mut tasks = Vec::new();
        for (path, entry) in &self.file_cache {
            if entry.dirty
                && !is_untitled(path)
                && !self.large.contains_key(path)
                && entry.stamp.is_some()
                && DiskStamp::of(path) == entry.stamp
            {
                if let Some(content) = entry.text() {
                    tasks.push(save_command(
                        Some(path.clone()),
                        save_file(path.clone(), content, self.save_format(entry.format)),
                    ));
                }
            }
        }
        tasks
    }

    fn auto_save_files(&mut self) -> Task<Message> {
        if !self.config.auto_save
            || self.autosaving > 0
            || self.last_edit.elapsed() < std::time::Duration::from_millis(700)
        {
            return Task::none();
        }
        let mut tasks = Vec::new();
        if let Some(path) = self
            .current_file
            .clone()
            .filter(|p| !is_untitled(p) && !self.large.contains_key(p))
        {
            if self.is_dirty && !self.external_warned && !self.changed_on_disk(&path) {
                let content = crate::editor::buffer_text(&self.editor.content);
                tasks.push(save_command(
                    Some(path.clone()),
                    save_file(path, content, self.save_format(self.format)),
                ));
            }
        }
        tasks.extend(self.save_cached_tabs());
        if tasks.is_empty() {
            return Task::none();
        }
        self.autosaving = tasks.len();
        Task::batch(tasks)
    }

    /// Whether the blame column is drawn for the current file.
    fn blame_shown(&self) -> bool {
        self.blame
            .as_ref()
            .is_some_and(|b| self.current_file.as_ref() == Some(&b.path))
    }

    /// The blame column of the current file, for the editor.
    fn blame_annotations(&self) -> Option<crate::code_editor::Annotations> {
        self.blame
            .as_ref()
            .filter(|b| self.current_file.as_ref() == Some(&b.path))
            .map(|b| b.annotations.clone())
    }

    /// The diff the clipboard keys act on: the tab in front.
    fn diff_viewer(&mut self) -> Option<&mut crate::diff_view::Viewer> {
        if !self.on_diff_tab() {
            return None;
        }
        let path = self.current_file.clone()?;
        self.diff_tabs.get_mut(&path).map(|tab| &mut tab.viewer)
    }

    fn diff_clipboard(&mut self, key: &str) -> Option<Task<Message>> {
        let viewer = self.diff_viewer()?;
        match key.to_ascii_lowercase().as_str() {
            "a" => {
                viewer.select_all();
                Some(Task::none())
            }
            "c" | "x" => viewer.selected_text().map(iced::clipboard::write),
            _ => None,
        }
    }

    fn on_diff_tab(&self) -> bool {
        self.current_file.as_deref().is_some_and(is_diff)
    }

    /// Keeps what a file holds now, before something rewrites it.
    fn remember_file(&mut self, path: PathBuf) {
        if let Ok(bytes) = file_io::read_bytes(&path) {
            self.work_undo.push((path, bytes));
            if self.work_undo.len() > 50 {
                self.work_undo.remove(0);
            }
            self.work_redo.clear();
        }
    }

    /// Ctrl+Z (or Ctrl+Y) in a diff: puts back the file as it was before the
    /// last rollback or arrow (or again after it).
    fn undo_work(&mut self, undo: bool) -> Task<Message> {
        // Only what happened to the files this diff is about.
        let files: Vec<PathBuf> = self
            .current_file
            .as_ref()
            .and_then(|p| self.diff_tabs.get(p))
            .and_then(|tab| tab.owner.as_ref())
            .map(|(_, _, files)| files.iter().flatten().cloned().collect())
            .unwrap_or_default();
        let in_scope = |path: &PathBuf| files.contains(path);
        let stack = if undo {
            &mut self.work_undo
        } else {
            &mut self.work_redo
        };
        let popped = stack
            .iter()
            .rposition(|(path, _)| in_scope(path))
            .map(|i| stack.remove(i));
        let Some((path, text)) = popped else {
            self.set_status(t!("status.nothing_to_undo"), false);
            return Task::none();
        };
        // What the file holds now is what the opposite key gives back.
        let now = file_io::read_bytes(&path).unwrap_or_default();
        let (other, label) = if undo {
            (&mut self.work_redo, t!("status.diff_undone"))
        } else {
            (&mut self.work_undo, t!("status.diff_redone"))
        };
        other.push((path.clone(), now));
        if let Err(e) = crate::fsutil::write_atomic(&path, &text, false) {
            self.set_status(format!("{}: {e}", t!("status.error")), true);
            return Task::none();
        }
        self.set_status(label, false);
        Task::none()
    }

    fn on_merge_tab(&self) -> bool {
        self.current_file
            .as_ref()
            .is_some_and(|p| self.merge_tabs.contains_key(p))
    }

    /// The buttons of the merge tab in front.
    fn merge_msg(&mut self, msg: crate::diff_view::merge::MergeMsg) -> Task<Message> {
        let Some(path) = self.current_file.clone() else {
            return Task::none();
        };
        let Some(tab) = self.merge_tabs.get_mut(&path) else {
            return Task::none();
        };
        // Typing in a result: the app's shortcuts stay out of it.
        if matches!(msg, crate::diff_view::merge::MergeMsg::Edit(..)) {
            self.merge_typing = true;
        }
        if matches!(msg, crate::diff_view::merge::MergeMsg::Apply) {
            if tab.merge.unresolved() > 0 {
                return Task::none();
            }
            let (repo, relative, text) = (tab.repo.clone(), tab.path.clone(), tab.merge.render());
            let Some((plugin, tab_id)) = tab.owner.clone() else {
                return Task::none();
            };
            // The merge was read from the disk: changes not saved in an
            // editor tab of the same file would be lost under it.
            if self.is_path_dirty(&repo.join(&relative)) {
                self.set_status(
                    t!("git.merge_buffer_unsaved", name = relative.as_str()),
                    true,
                );
                return Task::none();
            }
            // The file is written, the plugin does the rest.
            let file = repo.join(&relative);
            return Task::perform(
                plugins::merge_written(plugin, tab_id, file, text),
                move |result| Message::MergeApplied(path.clone(), result),
            );
        }
        tab.update(msg)
    }

    /// What the status bar offers: a language server.
    fn status_offer(&self) -> Option<(String, String, Message)> {
        self.server_offer.map(|info| {
            (
                t!("status.server_offer", language = info.label).to_string(),
                t!("prefs.server_install").to_string(),
                Message::InstallServer(info.binary),
            )
        })
    }

    /// The arrows of the diff tab in front.
    fn diff_nav(&mut self, nav: DiffNav) -> Task<Message> {
        use ardoise_plugin::protocol::DiffAction;
        let Some(path) = self.current_file.clone() else {
            return Task::none();
        };
        let Some(tab) = self.diff_tabs.get_mut(&path) else {
            return Task::none();
        };
        let (plugin, tab_id, files) = tab.owner.clone().unwrap_or_default();
        let file = tab.viewer.index.unwrap_or(0);
        let disk = files.get(file).cloned().flatten();
        // What the plugin is asked to do.
        let action = match nav {
            DiffNav::NextChange => return tab.viewer.next_change(),
            DiffNav::PrevChange => return tab.viewer.prev_change(),
            DiffNav::NextFile => return tab.viewer.next_file(crate::diff_view::Land::First),
            DiffNav::PrevFile => return tab.viewer.prev_file(crate::diff_view::Land::First),
            DiffNav::Pick(pick) => {
                tab.viewer.pick(pick);
                return Task::none();
            }
            DiffNav::Scrolled(top, height) => {
                tab.viewer.view_top.set(top as f32);
                tab.viewer.view_height.set(height as f32);
                return Task::none();
            }
            DiffNav::CopyPicked => {
                return match tab.viewer.selected_text() {
                    Some(text) => iced::clipboard::write(text),
                    None => Task::none(),
                };
            }
            DiffNav::PickAll => {
                tab.viewer.select_all();
                return Task::none();
            }
            DiffNav::ToggleSide => {
                tab.viewer.side_by_side = !tab.viewer.side_by_side;
                tab.viewer.selection = None;
                return tab.viewer.scroll_to_change();
            }
            DiffNav::OpenFile => {
                return match disk.filter(|f| f.is_file()) {
                    Some(f) => self.activate(f),
                    None => {
                        self.set_status(t!("git.file_gone"), true);
                        Task::none()
                    }
                };
            }
            DiffNav::StageBlock(_) | DiffNav::StageSelection => {
                let block = match nav {
                    DiffNav::StageBlock(b) => Some(b),
                    _ => None,
                };
                let Some((patch, reverse)) = tab.viewer.index_patch(block) else {
                    return Task::none();
                };
                DiffAction::ApplyToIndex { patch, reverse }
            }
            DiffNav::Apply(block) => {
                // What the file holds is kept: Ctrl+Z in the diff brings it back.
                if let Some(disk) = disk {
                    self.remember_file(disk);
                }
                DiffAction::Rollback { block }
            }
            DiffNav::ToggleBlanks => {
                tab.viewer.ignore_blanks = !tab.viewer.ignore_blanks;
                DiffAction::Options {
                    ignore_blanks: tab.viewer.ignore_blanks,
                    context: tab.viewer.context,
                }
            }
            DiffNav::CycleContext => {
                tab.viewer.cycle_context();
                DiffAction::Options {
                    ignore_blanks: tab.viewer.ignore_blanks,
                    context: tab.viewer.context,
                }
            }
            DiffNav::ToggleCompare => DiffAction::ToggleCompare,
        };
        crate::plugins::host::notify(
            &plugin,
            ardoise_plugin::protocol::methods::DIFF_ACTION,
            ardoise_plugin::protocol::DiffActionParams {
                tab: tab_id,
                file,
                action,
            },
        );
        Task::none()
    }

    /// Closes every tab but `keep` (none: all of them), asking once first if
    /// some of them hold unsaved changes.
    fn request_close_many(&mut self, keep: Option<PathBuf>) -> Task<Message> {
        let targets: Vec<PathBuf> = self
            .sidebar
            .files()
            .iter()
            .filter(|p| Some(*p) != keep.as_ref())
            .cloned()
            .collect();
        if targets.is_empty() {
            return Task::none();
        }
        // The tab that stays is the one in front.
        let show = match keep {
            Some(path) if self.current_file.as_ref() != Some(&path) => self.activate(path),
            _ => Task::none(),
        };
        self.confirm_close_many(targets, show)
    }

    /// Closes these tabs, after asking if some have unsaved changes.
    fn confirm_close_many(&mut self, targets: Vec<PathBuf>, show: Task<Message>) -> Task<Message> {
        let modified = targets.iter().filter(|p| self.is_path_dirty(p)).count();
        if modified > 0 {
            self.overlay = ActiveOverlay::ConfirmCloseMany(targets, modified);
            show
        } else {
            Task::batch([show, self.close_many(targets)])
        }
    }

    fn close_many(&mut self, paths: Vec<PathBuf>) -> Task<Message> {
        Task::batch(paths.into_iter().map(|path| self.close_path(path)))
    }

    /// Removes a tab and, if it was the active one, switches to its neighbour.
    fn close_path(&mut self, path: PathBuf) -> Task<Message> {
        let index = self.sidebar.files().iter().position(|p| *p == path);
        if self.preview_tab.as_ref() == Some(&path) {
            self.preview_tab = None;
        }
        // A file's tab can be opened again (Ctrl+Shift+T).
        if index.is_some() && !is_untitled(&path) && !is_virtual(&path) {
            self.closed_tabs.retain(|p| *p != path);
            self.closed_tabs.push(path.clone());
            if self.closed_tabs.len() > 50
                && is_untitled(&self.closed_tabs.remove(0))
                && !self.closed_untitled.is_empty()
            {
                self.closed_untitled.remove(0);
            }
        }
        // An untitled tab that held some text can be opened again too.
        if index.is_some() && is_untitled(&path) && !is_virtual(&path) {
            let kept = if self.current_file.as_ref() == Some(&path) {
                Some(crate::editor::buffer_text(&self.editor.content))
            } else {
                self.file_cache
                    .get(&path)
                    .and_then(|b| b.content.as_ref())
                    .map(crate::editor::buffer_text)
            };
            if let Some(text) = kept.filter(|t| !t.trim().is_empty()) {
                self.closed_untitled.push(text);
                if self.closed_untitled.len() > 20 {
                    self.closed_untitled.remove(0);
                }
                self.closed_tabs.push(path.clone());
            }
        }
        self.file_cache.remove(&path);
        self.undo_cache.remove(&path);
        self.large.remove(&path);
        // A diff or a conflict of a plugin: it hears the tab is gone.
        let owner = self
            .diff_tabs
            .remove(&path)
            .and_then(|t| t.owner.map(|(plugin, tab, _)| (plugin, tab)))
            .or_else(|| self.merge_tabs.remove(&path).and_then(|t| t.owner));
        if let Some((plugin, tab)) = owner {
            crate::plugins::host::notify(
                &plugin,
                ardoise_plugin::protocol::methods::TAB_CLOSED,
                ardoise_plugin::protocol::PanelParams { panel: tab },
            );
        }
        if let Some(tab) = self.plugins.tabs.remove(&path) {
            crate::plugins::host::notify(
                &tab.plugin,
                ardoise_plugin::protocol::methods::TAB_CLOSED,
                ardoise_plugin::protocol::PanelParams {
                    panel: tab.tab.clone(),
                },
            );
            self.plugins.views.remove(&(tab.plugin, tab.tab));
        }
        self.sidebar.remove_path(&path);
        // An untitled file is gone for good once closed; a saved one keeps
        // its bookmarks for the next time it is opened.
        if is_untitled(&path) {
            self.bookmarks.clear(&path);
        }

        let mut command = Task::none();
        if self.current_file.as_ref() == Some(&path) {
            self.editor = EditorState::new();
            self.reset_view_mode();
            self.current_file = None;
            self.is_dirty = false;
            self.undo_stack.clear();
            self.redo_stack.clear();
            self.undo_new_group = true;
            self.search.reset();
            self.format = TextFormat::default();
            self.disk_stamp = None;
            self.saved_fp = None;
            self.set_status(t!("status.ready"), false);
            let files = self.sidebar.files();
            let next = index
                .and_then(|i| files.get(i))
                .or_else(|| files.last())
                .cloned();
            command = match next {
                Some(next) => self.activate(next),
                None => self.ensure_tab(),
            };
        }
        // The session is written by the next tick (in the background), not
        // here: closing many tabs would otherwise write it once per tab.
        self.session_rev = self.rev.wrapping_sub(1);
        if index.is_some() {
            self.plugins_file_event(ardoise_plugin::manifest::Event::FileClosed, &path);
        }
        if index.is_some() && !is_untitled(&path) && !is_virtual(&path) {
            // The language servers that had it open let go of it.
            let closed = Task::future(crate::lsp::did_close(path)).discard();
            return Task::batch([command, closed]);
        }
        command
    }

    /// The editor is never left without a tab: when the last one is closed, a
    /// blank untitled one takes its place (text typed with nowhere to live
    /// would be lost).
    fn ensure_tab(&mut self) -> Task<Message> {
        if self.current_file.is_some() {
            return Task::none();
        }
        if let Some(first) = self.sidebar.files().first().cloned() {
            return self.activate(first);
        }
        self.untitled_counter = 1;
        self.load_buffer(untitled_path(1), CachedBuffer::blank());
        Task::none()
    }

    /// Whether `path` is one of the scratch files.
    fn is_scratch(&self, path: &Path) -> bool {
        crate::scratch::is_scratch(&self.scratch.dir, path)
    }

    /// The same for the tick of the clock: written off the interface thread,
    /// by the way of the auto-save.
    fn flush_scratch_later(&mut self) -> Task<Message> {
        let Some(path) = self.current_file.clone().filter(|p| self.is_scratch(p)) else {
            return Task::none();
        };
        if !self.is_dirty || self.autosaving > 0 || self.changed_on_disk(&path) {
            return Task::none();
        }
        // The tests have no runtime to run a task: it is written on the spot.
        if cfg!(test) {
            self.flush_scratch();
            return Task::none();
        }
        let content = crate::editor::buffer_text(&self.editor.content);
        self.autosaving = 1;
        save_command(
            Some(path.clone()),
            save_file(path, content, self.save_format(self.format)),
        )
    }

    /// Writes the scratch file in front if it was typed in: they are saved by
    /// themselves, with no question asked.
    fn flush_scratch(&mut self) {
        let Some(path) = self.current_file.clone().filter(|p| self.is_scratch(p)) else {
            return;
        };
        if !self.is_dirty || self.changed_on_disk(&path) {
            return;
        }
        let content = crate::editor::buffer_text(&self.editor.content);
        let Some(bytes) = self.format.encode_bytes_for(&content, &path) else {
            self.set_status(
                t!("status.err_encoding", name = self.format.encoding.label()),
                true,
            );
            return;
        };
        match crate::fsutil::write_atomic(&path, &bytes, false) {
            Ok(()) => {
                self.is_dirty = false;
                self.saved_fp = Some(fingerprint(&content));
                self.disk_stamp = DiskStamp::of(&path);
                self.external_warned = false;
                self.size_for = None;
            }
            Err(e) => self.set_status(format!("{}: {e}", t!("status.error")), true),
        }
    }

    /// Shows a scratch file in the editor (it has no tab of its own).
    fn open_scratch(&mut self, path: PathBuf) -> Task<Message> {
        if !path.is_file() {
            self.scratch.error = Some(t!("scratch.err_missing").to_string());
            self.scratch.refresh();
            return Task::none();
        }
        self.flush_scratch();
        self.activate(path)
    }

    /// Keeps the scratch files elsewhere: they move along with it.
    fn set_scratch_dir(&mut self, setting: String) -> Task<Message> {
        let setting = setting.trim().to_string();
        self.preferences.scratch_draft = setting.clone();
        let (old, new) = (self.scratch.dir.clone(), crate::scratch::dir_of(&setting));
        if old == new {
            self.config.scratch_dir = setting;
            self.config_dirty = true;
            return Task::none();
        }
        // An existing folder of other files would turn them all into scratches.
        if !crate::scratch::usable(&new) {
            self.preferences.scratch_draft = self.config.scratch_dir.clone();
            self.set_status(t!("scratch.err_not_empty"), true);
            return Task::none();
        }
        self.flush_scratch();
        // Their buffers are closed first: the files are about to move.
        let open: Vec<PathBuf> = self
            .file_cache
            .keys()
            .chain(self.current_file.as_ref())
            .filter(|p| self.is_scratch(p))
            .cloned()
            .collect();
        let mut tasks: Vec<Task<Message>> = open.into_iter().map(|p| self.close_path(p)).collect();
        match crate::scratch::relocate(&old, &new) {
            Ok(_) => {
                self.config.scratch_dir = setting;
                self.config_dirty = true;
                self.scratch.dir = new;
                self.scratch.error = None;
            }
            Err(e) => {
                self.preferences.scratch_draft = self.config.scratch_dir.clone();
                self.set_status(format!("{}: {e}", t!("status.error")), true);
            }
        }
        self.scratch.refresh();
        tasks.push(self.ensure_tab());
        Task::batch(tasks)
    }

    /// A file of the tree, clicked: it takes the preview tab (the file that
    /// was in it goes, unless it was edited); clicked twice, it stays.
    fn tree_open(&mut self, path: PathBuf) -> Task<Message> {
        self.tree_focus = true;
        self.projects.selected = Some(path.clone());
        let now = std::time::Instant::now();
        let twice = self.tree_click.as_ref().is_some_and(|(p, at)| {
            *p == path && now.duration_since(*at) < std::time::Duration::from_millis(450)
        });
        self.tree_click = Some((path.clone(), now));
        if twice {
            if self.preview_tab.as_ref() == Some(&path) {
                self.preview_tab = None;
            }
            return self.activate(path);
        }
        // Already a tab (the preview, or one that stays): it is shown.
        if self.sidebar.files().contains(&path) {
            return self.activate(path);
        }
        let dirty = self.dirty_files();
        if let Some(old) = self.preview_tab.take() {
            if !dirty.contains(&old) {
                if let Some(index) = self.sidebar.files().iter().position(|p| *p == old) {
                    self.preview_replaces = Some((old, index));
                }
            }
        }
        self.preview_tab = Some(path.clone());
        self.activate(path)
    }

    /// Once the file of the preview is in the editor, the one it replaces
    /// goes and it takes its place; once it is edited, it stays.
    fn settle_preview(&mut self) -> Task<Message> {
        let Some(preview) = self.preview_tab.clone() else {
            return Task::none();
        };
        if self.current_file.as_ref() != Some(&preview) {
            return Task::none();
        }
        let mut closing = Task::none();
        if self.is_dirty {
            self.preview_tab = None;
        }
        if let Some((old, index)) = self.preview_replaces.take() {
            if old != preview {
                // The language servers let go of the file that was replaced.
                closing = self.close_path(old);
                let files = self.sidebar.files();
                if let Some(at) = files.iter().position(|p| *p == preview) {
                    let to = index.min(files.len().saturating_sub(1));
                    self.sidebar.move_tab(at, to);
                }
            }
        }
        closing
    }

    /// The file in the editor goes first in the order of the ones seen.
    fn track_mru(&mut self) {
        if self.tab_switch.is_some() {
            return;
        }
        if let Some(current) = &self.current_file {
            if self.mru.first() != Some(current) {
                self.mru.retain(|p| p != current);
                self.mru.insert(0, current.clone());
                self.mru.truncate(200);
            }
        }
    }

    /// Ctrl+Tab: the open files in the order they were seen, the one in the
    /// editor first; each press while Ctrl is held goes one further.
    fn switch_tab(&mut self, forward: bool) -> Task<Message> {
        if self.tab_switch.is_none() {
            let open = self.sidebar.files();
            let mut order: Vec<PathBuf> = self
                .mru
                .iter()
                .filter(|p| open.contains(p))
                .cloned()
                .collect();
            let rest: Vec<PathBuf> = open
                .iter()
                .filter(|p| !order.contains(p))
                .cloned()
                .collect();
            order.extend(rest);
            self.tab_switch = Some((order, 0));
        }
        let Some((order, at)) = &mut self.tab_switch else {
            return Task::none();
        };
        if order.len() < 2 {
            return Task::none();
        }
        *at = if forward {
            (*at + 1) % order.len()
        } else {
            (*at + order.len() - 1) % order.len()
        };
        let target = order[*at].clone();
        self.activate(target)
    }

    /// Ctrl+Shift+T: the last tab closed that can still be read.
    fn reopen_tab(&mut self) -> Task<Message> {
        while let Some(path) = self.closed_tabs.pop() {
            if is_untitled(&path) {
                let Some(text) = self.closed_untitled.pop() else {
                    continue;
                };
                self.stash_current();
                self.untitled_counter += 1;
                let reopened = untitled_path(self.untitled_counter);
                let buffer = CachedBuffer {
                    content: Some(crate::code_editor::Content::with_text(&text)),
                    dirty: true,
                    ..CachedBuffer::blank()
                };
                self.load_buffer(reopened, buffer);
                return Task::none();
            }
            if self.sidebar.files().contains(&path) || !path.is_file() {
                continue;
            }
            return self.activate(path);
        }
        Task::none()
    }

    /// Scrolls the tree just enough to show the line picked.
    fn scroll_tree_to_selected(&self) -> Task<Message> {
        let Some(index) = self
            .projects
            .selected
            .as_ref()
            .and_then(|s| self.projects.index_of(s))
        else {
            return Task::none();
        };
        let row = crate::sidebar::TREE_ROW + 1.0;
        let (offset, height) = self.tree_view;
        let top = index as f32 * row;
        let y = if top < offset {
            top
        } else if height > 0.0 && top + row > offset + height {
            top + row - height
        } else {
            return Task::none();
        };
        iced::widget::operation::scroll_to(
            crate::sidebar::tree_id(),
            iced::widget::scrollable::AbsoluteOffset { x: 0.0, y },
        )
    }

    /// Keeps the projects and their open folders in the settings.
    fn save_projects(&mut self) {
        self.config.projects = self.projects.roots.clone();
        self.config.projects_open = self.projects.expanded.iter().cloned().collect();
        self.assign_project_colors();
        self.config_dirty = true;
    }

    /// Parks the active buffer (text, cursor, scroll, language, dirty flag,
    /// undo history) so another file can take the editor.
    fn stash_current(&mut self) {
        self.flush_scratch();
        if let Some(current) = self.current_file.clone() {
            let content = std::mem::take(&mut self.editor.content);
            self.file_cache.insert(
                current.clone(),
                CachedBuffer {
                    content: Some(content),
                    language: self.editor.language.clone(),
                    dirty: self.is_dirty,
                    stamp: self.disk_stamp,
                    format: self.format,
                    saved_fp: self.saved_fp,
                    view_mode: self.view_mode,
                },
            );
            self.undo_cache.insert(
                current,
                (
                    std::mem::take(&mut self.undo_stack),
                    std::mem::take(&mut self.redo_stack),
                ),
            );
            self.enforce_undo_budget();
        }
    }

    /// Puts a buffer in the editor and makes it the current tab.
    fn load_buffer(&mut self, path: PathBuf, buffer: CachedBuffer) {
        // Its tab is shown: its group opens.
        let project = self.tab_key(&path);
        self.folded_tabs.remove(&project);
        let content = buffer.content.unwrap_or_default();
        let first_line = first_line_of(&content);
        self.editor = EditorState {
            content,
            language: None,
        };
        self.editor
            .restore_language(buffer.language, &path, &first_line);
        self.reset_view_mode();
        self.view_mode = buffer.view_mode;
        if !self.is_scratch(&path) {
            self.sidebar.add_file(path.clone());
        }
        let (undo, redo) = self.undo_cache.remove(&path).unwrap_or_default();
        self.undo_stack = undo;
        self.redo_stack = redo;
        self.undo_new_group = true;
        self.pending_closers.clear();
        self.search.reset();
        self.format_error = None;
        self.show_error_panel = false;
        self.format = buffer.format;
        self.disk_stamp = buffer.stamp;
        self.saved_fp = buffer.saved_fp;
        self.external_warned = false;
        self.current_file = Some(path);
        self.is_dirty = buffer.dirty;
        self.text_rev += 1;
    }

    /// Makes a file just read the active tab.
    /// Opens the file a place names (a click in the terminal), at its line
    /// and column.
    fn open_place(&mut self, at: crate::places::Location) -> Task<Message> {
        let col = crate::places::byte_col(&at.path, at.line, at.col);
        self.jump_to(crate::definition::Hit {
            path: at.path,
            line: at.line.saturating_sub(1),
            col: col.saturating_sub(1),
            len: 0,
        })
    }

    fn open_loaded(&mut self, loaded: Loaded) {
        let path = loaded.path.clone();
        self.plugins_file_event(ardoise_plugin::manifest::Event::FileOpened, &path);
        self.pending_open = None;
        if self.current_file.as_ref() == Some(&path) {
            // Opening the file being edited again: bring it up to date, but
            // never throw away unsaved changes.
            if self.is_dirty {
                self.set_status(t!("status.already_open_modified"), false);
            } else {
                self.apply_reload(loaded);
            }
            return;
        }
        let cached = self.file_cache.remove(&path);
        self.stash_current();
        let mut warn_changed = false;
        let buffer = match cached {
            // A background tab with unsaved edits wins over the disk.
            Some(c) if c.dirty && c.content.is_some() => {
                warn_changed = c.stamp.is_some() && loaded.stamp != c.stamp;
                c
            }
            other => {
                // The tab changed on disk while in the background (a Git
                // rollback, say): what it showed stays one Ctrl+Z away.
                if let Some(old) = other.as_ref().and_then(|c| c.content.as_ref()) {
                    let old_text = crate::editor::buffer_text(old);
                    if old_text != loaded.text {
                        let snapshot = Snapshot {
                            text: old_text,
                            cursor: old.cursor_position(),
                            extras: old.extra_carets(),
                        };
                        let (undo, redo) = self.undo_cache.entry(path.clone()).or_default();
                        undo.push_back(snapshot);
                        trim_history(undo);
                        redo.clear();
                    }
                }
                let view_mode = other.as_ref().map(|c| c.view_mode);
                let language = other.and_then(|c| c.language);
                let mut buffer = CachedBuffer::from_loaded(loaded, language);
                if let Some(mode) = view_mode {
                    buffer.view_mode = mode;
                }
                buffer
            }
        };
        if !self.is_scratch(&path) {
            self.config.add_recent(&path);
            self.config_dirty = true;
        }
        // A file too large to open whole came as its first part.
        self.large.remove(&path);
        if let Ok(meta) = std::fs::metadata(&path) {
            if meta.is_file() && meta.len() > MAX_OPEN_BYTES {
                let len = buffer
                    .content
                    .as_ref()
                    .map_or(0, |c| crate::editor::buffer_text(c).len() as u64);
                self.large.insert(
                    path.clone(),
                    LargeView {
                        start: 0,
                        end: len,
                        size: meta.len(),
                    },
                );
            }
        }
        if self.config.detect_indent && !self.indents.contains_key(&path) {
            let text = buffer
                .content
                .as_ref()
                .map(crate::editor::buffer_text)
                .unwrap_or_default();
            if let Some(found) = text_ops::detect_indent(&text, self.config.tab_width) {
                self.indents.insert(path.clone(), found);
            }
        }
        self.load_buffer(path.clone(), buffer);
        if warn_changed {
            self.external_warned = true;
            self.set_status(t!("status.changed_on_disk"), true);
        } else if self.has_very_long_line() {
            self.set_status(t!("status.long_line_hint"), false);
        } else {
            self.set_status(t!("status.file_opened"), false);
            self.offer_language_server(&path);
        }
    }

    /// Whether a line is so long (minified JSON, say) that typing on it is
    /// slow: the status bar then suggests formatting the file.
    fn has_very_long_line(&self) -> bool {
        self.editor
            .language
            .as_deref()
            .is_some_and(crate::formatter::has_formatter)
            && self
                .editor
                .content
                .lines()
                .any(|line| line.len() > LONG_LINE_BYTES)
    }

    /// Replaces the text of the active tab by a newer version from disk,
    /// keeping the undo history (the old text is one Ctrl+Z away), the cursor,
    /// the scroll position, the language and the view mode.
    fn apply_reload(&mut self, loaded: Loaded) {
        if self.current_file.as_ref() != Some(&loaded.path) {
            return;
        }
        if self.is_dirty {
            // Typing started while the file was being read: the disk version
            // is not applied, and saving will ask first.
            self.external_warned = true;
            self.set_status(t!("status.changed_on_disk"), true);
            return;
        }
        let old = crate::editor::buffer_text(&self.editor.content);
        if old != loaded.text {
            let (line, col) = self.editor.content.cursor_position();
            self.push_undo();
            self.undo_new_group = true;
            self.replace_buffer_text(&loaded.text);
            self.restore_cursor(line, col);
            self.text_rev += 1;
        }
        self.saved_fp = Some(fingerprint(&loaded.text));
        self.format = loaded.format;
        self.disk_stamp = loaded.stamp;
        self.external_warned = false;
        self.size_for = None;
        self.set_status(t!("status.file_reloaded"), false);
    }

    /// Switches to a tab: instantly if its buffer is cached, from disk otherwise.
    fn activate(&mut self, path: PathBuf) -> Task<Message> {
        if self.current_file.as_ref() == Some(&path) {
            return Task::none();
        }
        // A read still on its way for another tab is abandoned: when it
        // arrives it must not take the user there.
        if let Some(previous) = self.pending_open.take().filter(|p| *p != path) {
            self.abandoned_reads.insert(previous);
        }
        self.abandoned_reads.remove(&path);
        self.pending_open = Some(path.clone());
        let cached = self.file_cache.get(&path);
        if let Some(entry) = cached.filter(|e| e.content.is_some()) {
            // A saved file may have changed since this copy was made.
            let changed =
                !is_untitled(&path) && entry.stamp.is_some() && DiskStamp::of(&path) != entry.stamp;
            // Gone from the disk (deleted or moved): the copy is kept, as it
            // was, and the status says so; saving recreates the file.
            let deleted = changed && DiskStamp::of(&path).is_none();
            if changed && !deleted && !entry.dirty {
                return Task::perform(read_file(path, false), Message::TabRead);
            }
            let Some(entry) = self.file_cache.remove(&path) else {
                return Task::none();
            };
            self.pending_open = None;
            self.stash_current();
            self.load_buffer(path.clone(), entry);
            if changed {
                // Modified here and there: saving will ask before overwriting.
                self.external_warned = true;
                let message = if deleted {
                    t!("status.deleted_on_disk")
                } else {
                    t!("status.changed_on_disk")
                };
                self.set_status(message, true);
            } else {
                self.set_status(t!("status.file_opened"), false);
                self.offer_language_server(&path);
            }
            Task::none()
        } else if is_untitled(&path) {
            self.pending_open = None;
            self.stash_current();
            self.load_buffer(path, CachedBuffer::blank());
            Task::none()
        } else {
            // TabRead parks the current buffer once the read succeeds.
            Task::perform(read_file(path, false), Message::TabRead)
        }
    }

    /// Whether the file changed on disk since this tab read or wrote it.
    fn changed_on_disk(&self, path: &Path) -> bool {
        !is_untitled(path)
            && self.disk_stamp.is_some()
            && DiskStamp::of(path).is_some_and(|now| Some(now) != self.disk_stamp)
    }

    /// Notices edits made to the current file by other programs: a clean
    /// buffer is reloaded, a modified one is warned about (saving then asks
    /// before overwriting).
    fn check_external_change(&mut self) -> Task<Message> {
        let Some(path) = self.current_file.clone().filter(|p| !is_untitled(p)) else {
            return Task::none();
        };
        let Some(now) = DiskStamp::of(&path) else {
            // Deleted or moved away: saving would silently recreate it.
            if self.disk_stamp.is_some() && !self.external_warned {
                self.external_warned = true;
                self.set_status(t!("status.deleted_on_disk"), true);
            }
            return Task::none();
        };
        if Some(now) == self.disk_stamp {
            return Task::none();
        }
        if self.is_dirty {
            if !self.external_warned {
                self.external_warned = true;
                self.set_status(t!("status.changed_on_disk"), true);
            }
            return Task::none();
        }
        if let Some(view) = self.large.get(&path).copied() {
            if std::fs::metadata(&path).is_ok_and(|m| m.len() > MAX_OPEN_BYTES) {
                // A big part is read: not more than once in a while.
                let waited = self.large_refreshed.map(|t| t.elapsed());
                if waited.is_some_and(|w| w < std::time::Duration::from_secs(5)) {
                    return Task::none();
                }
                self.large_refreshed = Some(std::time::Instant::now());
                self.disk_stamp = Some(now);
                return Task::perform(
                    async move {
                        let read = path.clone();
                        let result = tokio::task::spawn_blocking(move || {
                            file_io::read_window(&read, view.start, LARGE_WINDOW)
                        })
                        .await
                        .unwrap_or_else(|e| Err(e.to_string()));
                        (path, result)
                    },
                    |(path, result)| Message::LargeRefreshed(path, result),
                );
            }
            // Back under the limit: it opens whole again.
            self.large.remove(&path);
        }
        // The new state becomes the baseline only once it has been applied.
        Task::perform(read_file(path, false), Message::FileReloaded)
    }

    fn refresh_file_size(&mut self) {
        if self.size_for != self.current_file {
            self.size_for = self.current_file.clone();
            self.file_size = self
                .current_file
                .as_ref()
                .filter(|p| !is_untitled(p))
                .and_then(|p| std::fs::metadata(p).ok())
                .map(|m| m.len());
        }
    }

    /// The preview offered by the current buffer, if any.
    fn preview_kind(&self) -> Option<PreviewKind> {
        preview::kind_for(self.editor.language.as_deref())
    }

    /// Back to the plain editor: called whenever the buffer is replaced by a
    /// different file, since folded paths and the mode belong to one document.
    fn reset_view_mode(&mut self) {
        self.view_mode = ViewMode::Raw;
        self.json_collapsed = Default::default();
        self.preview_rev += 1;
        self.preview = PreviewDoc::None;
    }

    /// Reparses the buffer for the preview pane, but only when it is visible
    /// and its text may have changed since the last parse.
    fn refresh_preview(&mut self) {
        let Some(kind) = self.preview_kind() else {
            self.preview = PreviewDoc::None;
            return;
        };
        if self.view_mode == ViewMode::Raw {
            return;
        }
        if !matches!(self.preview, PreviewDoc::None) && self.preview_text_rev == self.text_rev {
            return;
        }

        let src = crate::editor::buffer_text(&self.editor.content);
        self.preview = match kind {
            PreviewKind::Markdown => {
                PreviewDoc::Markdown(std::sync::Arc::new(crate::preview::markdown::parse(&src)))
            }
            PreviewKind::Json => match crate::preview::json::parse(&src) {
                Ok(value) => PreviewDoc::Json(std::sync::Arc::new(value)),
                Err(e) => PreviewDoc::JsonError(e),
            },
        };
        self.preview_text_rev = self.text_rev;
        self.preview_rev += 1;
    }

    /// Shows a formatter's failure: the whole output in the panel, a short
    /// note in the status bar.
    fn show_format_error(&mut self, error: String) {
        let short = crate::formatter::short_error(&error);
        self.format_error = Some(error);
        self.show_error_panel = true;
        self.set_status(format!("{}: {short}", t!("status.format_error")), true);
    }

    /// The file the formatter should believe it is formatting, so it finds
    /// the project's configuration next to it. `None` for an unsaved buffer.
    /// The result of formatting the whole file: put in the text if the text
    /// is still the one that was sent.
    fn file_formatted(&mut self, job: FormatJob, result: Result<String, String>) -> Task<Message> {
        self.is_formatting = false;
        // The user moved on, or kept typing: the result describes text
        // that is no longer there.
        let current = self.current_file == job.path
            && fingerprint(&crate::editor::buffer_text(&self.editor.content)) == job.source;
        if !current {
            self.set_status(t!("status.format_stale"), true);
            return Task::none();
        }
        match result {
            Ok(formatted) => {
                let (line, col) = self.editor.content.cursor_position();
                self.format_error = None;
                self.show_error_panel = false;
                self.set_status(t!("status.formatted"), false);
                let formatted = TextFormat::decode(&formatted).0;
                return self.apply_text_edit(formatted, line, col, None);
            }
            Err(e) => self.show_format_error(e),
        }
        Task::none()
    }

    /// Opens a file named on the command line, at its line when it has one.
    fn open_target(&self, target: crate::ipc::Target) -> Task<Message> {
        let path = target.path.clone();
        Task::perform(read_file(path, true), move |result| {
            Message::TargetOpened(target.clone(), result)
        })
    }

    /// The name (letters, digits and `_`) the cursor is in or just after.
    fn word_at_cursor(&self) -> String {
        let (line, col) = self.editor.content.cursor_position();
        let Some(text) = self.editor.content.line(line) else {
            return String::new();
        };
        let col = floor_boundary(&text, col);
        let word = |c: char| c.is_alphanumeric() || c == '_';
        let start = text[..col]
            .char_indices()
            .rev()
            .take_while(|(_, c)| word(*c))
            .last()
            .map_or(col, |(i, _)| i);
        let end = text[col..]
            .char_indices()
            .take_while(|(_, c)| word(*c))
            .last()
            .map_or(col, |(i, c)| col + i + c.len_utf8());
        text[start..end].to_string()
    }

    /// The answer to a rename: the edits go into the editor (one Ctrl+Z),
    /// into the tabs, and on disk for the other files (undone from the
    /// actions).
    fn renamed(
        &mut self,
        source: u64,
        result: Result<Vec<crate::lsp::FileEdits>, String>,
    ) -> Task<Message> {
        let files = match result {
            Ok(files) => files,
            Err(e) => {
                self.set_status(format!("{}: {e}", t!("status.error")), true);
                return Task::none();
            }
        };
        let mut outcome = RenameOutcome::default();
        // The files nothing holds open are read and written in the background.
        let mut on_disk: Vec<(PathBuf, Vec<crate::lint::Edit>)> = Vec::new();
        for file in files {
            let fix = crate::lint::Fix {
                title: String::new(),
                edits: file.edits.clone(),
                safe: true,
            };
            if self.current_file.as_ref() == Some(&file.path) {
                let old = crate::editor::buffer_text(&self.editor.content);
                if fingerprint(&old) != source {
                    outcome.skipped += 1;
                    continue;
                }
                let (new, _) = crate::lint::apply(&old, &[&fix]);
                let (line, col) = self.editor.content.cursor_position();
                let _ = self.apply_text_edit(new, line, col, None);
                outcome.places += file.edits.len();
                outcome.changed += 1;
                continue;
            }
            let Some(text) = self.file_cache.get(&file.path).and_then(CachedBuffer::text) else {
                on_disk.push((file.path, file.edits));
                continue;
            };
            if self.is_path_dirty(&file.path) {
                outcome.skipped += 1;
                continue;
            }
            let (new, _) = crate::lint::apply(&text, &[&fix]);
            if new != text && self.put_text(&file.path, &finder::Holder::Tab, &text, &new) {
                outcome.places += file.edits.len();
                outcome.changed += 1;
                outcome.written.push(finder::Written {
                    path: file.path.clone(),
                    before: text,
                    after: new,
                });
            }
        }
        if on_disk.is_empty() {
            return self.rename_done(outcome, Vec::new());
        }
        // The tests have no runtime to run a task: it is done on the spot.
        if cfg!(test) {
            let disk = rename_on_disk(on_disk);
            return self.rename_done(outcome, disk);
        }
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || rename_on_disk(on_disk))
                    .await
                    .unwrap_or_default()
            },
            move |disk| Message::RenameDone(Box::new(outcome.clone()), disk),
        )
    }

    /// The edits of a rename are made in the files on disk too: it is told.
    fn rename_done(&mut self, mut outcome: RenameOutcome, disk: Vec<DiskEdited>) -> Task<Message> {
        for w in disk {
            self.bookmarks.remap(&w.path, &w.before, &w.after);
            outcome.places += w.places;
            outcome.changed += 1;
            outcome.written.push(finder::Written {
                path: w.path,
                before: w.before,
                after: w.after,
            });
        }
        if !outcome.written.is_empty() {
            self.finder.undo = std::mem::take(&mut outcome.written);
        }
        let files = finder::files_text(outcome.changed);
        let places = outcome.places;
        let mut message = t!("rename.done", count = places, files = files.as_str()).to_string();
        if outcome.skipped > 0 {
            message = format!("{message}. {}", t!("rename.skipped", n = outcome.skipped));
        }
        self.set_status(message, outcome.skipped > 0);
        Task::none()
    }

    /// What the language server says of the name under the pointer.
    fn symbol_card(&self, dark: bool) -> Option<(iced::Point, Element<'_, Message>)> {
        let (at, said) = self.symbol_card.as_ref()?;
        let lines: Vec<&str> = said.lines().take(24).collect();
        let x = at.x.min(self.config.window_width - 600.0).max(4.0);
        Some((
            iced::Point::new(x, at.y),
            container(
                text(lines.join("\n"))
                    .size(12.5)
                    .font(se_theme::MONO)
                    .shaping(iced::widget::text::Shaping::Advanced),
            )
            .padding(10)
            .max_width(580)
            .style(se_theme::card(dark))
            .into(),
        ))
    }

    fn format_target(&self) -> Option<PathBuf> {
        self.current_file.clone().filter(|p| !is_untitled(p))
    }

    /// Opens the "Save as" dialog; the chosen path comes back as `SaveAsChosen`.
    fn start_save_as(&self, origin: Option<PathBuf>) -> Task<Message> {
        let suggestion = origin.clone().filter(|p| !is_untitled(p));
        let name = origin.as_deref().map(file_label);
        Task::perform(pick_save_path(suggestion, name), move |result| {
            Message::SaveAsChosen(origin.clone(), result)
        })
    }
}

/// The text of the buffer as of a stamp.
type SharedText = ((u64, u64), std::sync::Arc<String>);

/// What a rename did so far.
#[derive(Debug, Clone, Default)]
pub struct RenameOutcome {
    places: usize,
    changed: usize,
    skipped: usize,
    written: Vec<finder::Written>,
}

/// A file on disk that a rename edited.
#[derive(Debug, Clone)]
pub struct DiskEdited {
    path: PathBuf,
    before: String,
    after: String,
    places: usize,
}

/// Makes the edits of a rename in files nothing holds open: each is read
/// once, and written back in the encoding it had. One that cannot be read or
/// written is left as it is.
fn rename_on_disk(files: Vec<(PathBuf, Vec<crate::lint::Edit>)>) -> Vec<DiskEdited> {
    let mut out = Vec::new();
    for (path, edits) in files {
        let Some((text, encoding)) = std::fs::read(&path)
            .ok()
            .and_then(|b| crate::finder::decode_file(&b))
        else {
            continue;
        };
        let fix = crate::lint::Fix {
            title: String::new(),
            edits: edits.clone(),
            safe: true,
        };
        let (after, _) = crate::lint::apply(&text, &[&fix]);
        if after == text {
            continue;
        }
        let Some(bytes) = encoding.encode(&after) else {
            continue;
        };
        if crate::fsutil::write_atomic(&path, &bytes, false).is_err() {
            continue;
        }
        out.push(DiskEdited {
            path,
            before: text,
            after,
            places: edits.len(),
        });
    }
    out
}

/// A text to paste in a terminal without its control characters (the escape
/// that would close a bracketed paste, Ctrl+C…): only tab and line breaks stay.
fn paste_text_safe(text: &str) -> String {
    text.chars()
        .filter(|c| !c.is_control() || matches!(c, '\t' | '\n' | '\r'))
        .collect()
}

/// The first line of the buffer (capped), to recognize a `#!` script.
fn first_line_of(content: &crate::code_editor::Content) -> String {
    content
        .line(0)
        .map(|l| l.chars().take(200).collect())
        .unwrap_or_default()
}

/// The largest char boundary of `s` at or before `index`.
/// The byte index of the `n`th character of `text` (its end if shorter).
fn byte_of_char(text: &str, n: usize) -> usize {
    text.char_indices().nth(n).map_or(text.len(), |(i, _)| i)
}

fn floor_boundary(s: &str, index: usize) -> usize {
    let mut index = index.min(s.len());
    while !s.is_char_boundary(index) {
        index -= 1;
    }
    index
}

/// A selection over several lines: (line, byte column) of both ends.
#[derive(Debug, Clone, Copy)]
struct Span {
    start: (usize, usize),
    end: (usize, usize),
}

impl Span {
    /// The lines a line command touches: a selection ending at the very
    /// start of a line doesn't include it.
    fn lines(self) -> (usize, usize) {
        let last = if self.end.1 == 0 && self.end.0 > self.start.0 {
            self.end.0 - 1
        } else {
            self.end.0
        };
        (self.start.0, last)
    }
}

fn nth_line(text: &str, index: usize) -> &str {
    text.split('\n').nth(index).unwrap_or("")
}

/// A column of `old_line` as it reads in `new_line` after an edit that
/// changed the line's length at its start.
fn remap_col(old_line: &str, new_line: &str, col: usize) -> usize {
    if col == 0 {
        return 0;
    }
    let delta = new_line.len() as isize - old_line.len() as isize;
    let col = (col as isize + delta).clamp(0, new_line.len() as isize) as usize;
    floor_boundary(new_line, col)
}

fn remap_span(old: &str, new: &str, span: Span, line_shift: isize) -> Span {
    let map = |(line, col): (usize, usize)| {
        let new_line = (line as isize + line_shift).max(0) as usize;
        (
            new_line,
            remap_col(nth_line(old, line), nth_line(new, new_line), col),
        )
    };
    Span {
        start: map(span.start),
        end: map(span.end),
    }
}

// ─── Editor pane ────────────────────────────────────────────────────────────

impl ArdoiseApp {
    /// The editor pane: the text editor alone, the preview, or both side by
    /// side (the mode switch lives in the panel's toolbar).
    fn editor_pane(
        &self,
        kind: Option<PreviewKind>,
        mode: ViewMode,
        dark: bool,
    ) -> Element<'_, Message> {
        if let Some(tab) = self
            .current_file
            .as_ref()
            .and_then(|p| self.merge_tabs.get(p))
        {
            return tab.view(dark);
        }
        if let Some(tab) = self
            .current_file
            .as_ref()
            .and_then(|p| self.plugins.tabs.get(p))
        {
            return self.plugin_tab_view(tab, dark);
        }
        if let Some(tab) = self
            .current_file
            .as_ref()
            .and_then(|p| self.diff_tabs.get(p))
        {
            return crate::diff_view::view::view(
                &tab.viewer,
                &crate::diff_view::view::Nav::tab(),
                dark,
                t!("git.no_diff").to_string(),
            );
        }
        let search = self.show_search.then_some(&self.search);
        let marks = self.bookmark_lines();
        // The context menu's actions (select/cut/copy/paste/format) all act on
        // the raw buffer: it wraps the editor only — not the preview beside
        // it, and not the preview alone, where it would act on text the user
        // can't see.
        let editor = || {
            self.with_context_menu(
                self.editor.view(
                    &self.config,
                    self.indent().1,
                    search,
                    marks,
                    self.plugin_run_marks(),
                    self.blame_annotations(),
                    self.change_paint(),
                    self.problem_paint(dark),
                    self.completion_hook(),
                ),
                dark,
            )
        };
        if kind.is_none() {
            return editor();
        }

        match mode {
            ViewMode::Raw => editor(),
            ViewMode::Preview => self.preview_view(dark),
            ViewMode::Split => row![
                container(editor()).width(Length::FillPortion(54)),
                container(Space::new().width(1))
                    .height(Length::Fill)
                    .style(se_theme::rule(dark)),
                container(self.preview_view(dark)).width(Length::FillPortion(46)),
            ]
            .height(Length::Fill)
            .into(),
        }
    }

    /// The 40px bar above the editor: file name and folder on the left, the
    /// Raw/Preview/Split switch (Markdown, JSON) on the right.
    fn toolbar_view(
        &self,
        kind: Option<PreviewKind>,
        mode: ViewMode,
        dark: bool,
    ) -> Element<'_, Message> {
        let name = self
            .current_file
            .as_deref()
            .map(file_label)
            .unwrap_or_else(|| t!("app.untitled").to_string());
        let dir = self
            .current_file
            .as_deref()
            .filter(|p| !is_untitled(p))
            .and_then(Path::parent)
            .map(short_dir)
            .unwrap_or_default();
        // A file a plugin brought (a copy of a remote one) says where it
        // comes from.
        let dir = self
            .current_file
            .as_ref()
            .and_then(|p| self.plugins.origins.get(p).cloned())
            .unwrap_or(dir);

        let on_diff = self.current_file.as_deref().is_some_and(is_virtual);
        let large_bar = |view: LargeView| -> Element<'_, Message> {
            let step = |label: String, to: LargeMove| {
                button(text(label).size(12))
                    .padding([2, 8])
                    .on_press(Message::LargeMove(to))
                    .style(
                        (se_theme::GhostButton {
                            dark,
                            active: false,
                        })
                        .style(),
                    )
            };
            let (from, to, size) = (
                megabytes(view.start),
                megabytes(view.end),
                megabytes(view.size),
            );
            let banner = t!("large.banner", from = from, to = to, size = size).to_string();
            row![
                text(banner).size(12).color(se_theme::muted_text(dark)),
                step(t!("large.first").to_string(), LargeMove::First),
                step(t!("large.prev").to_string(), LargeMove::Previous),
                step(t!("large.next").to_string(), LargeMove::Next),
                step(t!("large.last").to_string(), LargeMove::Last),
            ]
            .spacing(6)
            .align_y(Alignment::Center)
            .into()
        };
        let right: Element<Message> = match kind {
            _ if self.large_view().is_some() => {
                large_bar(self.large_view().expect("checked just above"))
            }
            _ if on_diff => Space::new().into(),
            Some(kind) => crate::preview::segmented(kind, mode, dark),
            None => Space::new().into(),
        };

        row![
            text(name)
                .size(13)
                .font(se_theme::UI_SEMIBOLD)
                .shaping(iced::widget::text::Shaping::Advanced),
            text(dir)
                .size(12)
                .color(se_theme::muted_text(dark))
                .shaping(iced::widget::text::Shaping::Advanced),
            Space::new().width(Length::Fill),
            right,
        ]
        .spacing(10)
        .padding([0, 14])
        .height(40)
        .align_y(Alignment::Center)
        .into()
    }

    /// Wraps the editor with the right-click menu.
    fn with_context_menu<'a>(
        &self,
        editor_widget: Element<'a, Message>,
        dark: bool,
    ) -> Element<'a, Message> {
        let lbl_select_all = t!("ctx.select_all").to_string();
        let lbl_cut = t!("ctx.cut").to_string();
        let lbl_copy = t!("ctx.copy").to_string();
        let lbl_paste = t!("ctx.paste").to_string();
        let lbl_delete = t!("ctx.delete").to_string();
        let lbl_rename = t!("ctx.rename").to_string();
        let sh_rename = self.config.shortcuts.rename_symbol.display();
        let lbl_split_right = t!("menu.view_split_right").to_string();
        let lbl_split_down = t!("menu.view_split_down").to_string();
        let lbl_split_close = t!("menu.view_split_close").to_string();
        let is_split = self.split.is_some();
        let lbl_lint_fix = t!("menu.lint_fix").to_string();
        let lbl_lint_now = t!("menu.lint_check").to_string();
        let has_problems = !self.lint.diagnostics.is_empty();
        let lbl_format_sel = t!("ctx.format_selection").to_string();
        let lbl_format_all = t!("ctx.format_all").to_string();
        let has_fmt_ctx = self
            .editor
            .language
            .as_deref()
            .map(crate::formatter::can_format)
            .unwrap_or(false);
        let has_sel_ctx = self.selection_info.is_some();
        // What can act now: an entry that cannot is off.
        let ed = self.menu_editor();
        let can_rename = ed.can_edit && self.has_language_server();
        let sh_select_all = self.config.shortcuts.select_all.display();
        let sh_format_all = self.config.shortcuts.format_code.display();
        // Cut/Copy/Paste are the platform's own keys, not configurable.
        let clip = |letter: &str| {
            if cfg!(target_os = "macos") {
                format!("⌘{letter}")
            } else {
                format!("Ctrl+{letter}")
            }
        };
        let (sh_cut, sh_copy, sh_paste) = (clip("X"), clip("C"), clip("V"));
        let bookmark_state = self.bookmark_state();
        let sh_bookmark = self.config.shortcuts.toggle_bookmark.display();

        let minimap_width = if self.config.show_minimap {
            crate::code_editor::MINIMAP_WIDTH
        } else {
            0.0
        };
        let gutter_menu = self.gutter_click;
        let blame_on = self.blame_shown();
        // The options of the blame are the Git plugin's (it knows how they
        // are set): each one switches.
        let lbl_previous = t!("git.blame_previous").to_string();
        let blame_items = [
            (t!("git.blame_blanks").to_string(), BlameOption::Blanks),
            (t!("git.blame_moves").to_string(), BlameOption::MovesInFile),
            (t!("git.blame_across").to_string(), BlameOption::MovesAcross),
            (t!("git.blame_hide").to_string(), BlameOption::HideLine),
            (
                t!("git.blame_show_hidden").to_string(),
                BlameOption::ShowHidden,
            ),
        ];
        let lbl_blame = t!("git.blame").to_string();
        let lbl_blame_close = t!("git.blame_close").to_string();
        let plugin_menus = self.plugin_menus();
        let menu = crate::pointer_area::context_menu(editor_widget, move || {
            use crate::menu_bar::{disabled_item, item, separator};
            if gutter_menu {
                let entry = if blame_on {
                    item(lbl_blame_close.clone(), None, dark, Message::CloseBlame)
                } else {
                    item(lbl_blame.clone(), None, dark, Message::BlameFile)
                };
                let mut entries = column![entry]
                    .spacing(1)
                    .padding(6)
                    .width(Length::Fixed(264.0));
                if blame_on {
                    entries = entries.push(item(
                        lbl_previous.clone(),
                        None,
                        dark,
                        Message::BlamePrevious,
                    ));
                    for (label, option) in &blame_items {
                        entries = entries.push(item(
                            label.clone(),
                            None,
                            dark,
                            Message::BlameOption(*option),
                        ));
                    }
                }
                return container(entries).style(crate::theme::card(dark)).into();
            }
            let fmt_sel_el: Element<'static, Message> = if has_fmt_ctx && has_sel_ctx {
                item(lbl_format_sel.clone(), None, dark, Message::FormatSelection)
            } else {
                disabled_item(lbl_format_sel.clone(), dark)
            };
            let fmt_all_el: Element<'static, Message> = if has_fmt_ctx {
                item(
                    lbl_format_all.clone(),
                    Some(sh_format_all.clone()),
                    dark,
                    Message::FormatFile,
                )
            } else {
                disabled_item(lbl_format_all.clone(), dark)
            };
            // An entry that cannot act now is shown off (and does nothing).
            let entry = |on: bool,
                         label: &String,
                         shortcut: Option<String>,
                         message: Message|
             -> Element<'static, Message> {
                if on {
                    item(label.clone(), shortcut, dark, message)
                } else {
                    disabled_item(label.clone(), dark)
                }
            };
            let ctx_items: Vec<Element<'static, Message>> = vec![
                entry(
                    ed.can_edit && ed.has_selection,
                    &lbl_cut,
                    Some(sh_cut.clone()),
                    Message::ContextCut,
                ),
                entry(
                    ed.has_selection,
                    &lbl_copy,
                    Some(sh_copy.clone()),
                    Message::ContextCopy,
                ),
                entry(
                    ed.can_edit,
                    &lbl_paste,
                    Some(sh_paste.clone()),
                    Message::ContextPaste,
                ),
                entry(
                    ed.can_edit && ed.has_selection,
                    &lbl_delete,
                    None,
                    Message::ContextDelete,
                ),
                entry(
                    can_rename,
                    &lbl_rename,
                    Some(sh_rename.clone()),
                    Message::RenameSymbol,
                ),
                entry(
                    ed.has_file,
                    &lbl_select_all,
                    Some(sh_select_all.clone()),
                    Message::SelectAll,
                ),
                separator(dark),
                fmt_sel_el,
                fmt_all_el,
                separator(dark),
            ];
            let mut ctx_items = ctx_items;
            ctx_items.extend(crate::menu_bar::bookmark_items(
                bookmark_state,
                dark,
                Some(sh_bookmark.clone()),
            ));
            let ours: Vec<_> = plugin_menus
                .iter()
                .filter(|m| m.menu == ardoise_plugin::manifest::Menu::Editor)
                .collect();
            if !ours.is_empty() {
                ctx_items.push(separator(dark));
                for m in ours {
                    ctx_items.push(item(
                        m.label.clone(),
                        None,
                        dark,
                        Message::Plugin(crate::plugins::PluginMsg::Run(
                            m.plugin.clone(),
                            m.command.clone(),
                        )),
                    ));
                }
            }
            ctx_items.push(separator(dark));
            ctx_items.push(entry(ed.can_check, &lbl_lint_now, None, Message::LintNow));
            ctx_items.push(if has_problems {
                item(lbl_lint_fix.clone(), None, dark, Message::LintFixAll)
            } else {
                disabled_item(lbl_lint_fix.clone(), dark)
            });
            ctx_items.push(separator(dark));
            ctx_items.push(item(
                lbl_split_right.clone(),
                None,
                dark,
                Message::Split(true),
            ));
            ctx_items.push(item(
                lbl_split_down.clone(),
                None,
                dark,
                Message::Split(false),
            ));
            if is_split {
                ctx_items.push(item(
                    lbl_split_close.clone(),
                    None,
                    dark,
                    Message::CloseSplit,
                ));
            }
            container(
                column(ctx_items)
                    .spacing(1)
                    .padding(6)
                    .width(Length::Fixed(264.0)),
            )
            .style(crate::theme::card(dark))
            .into()
        });
        crate::floating::RightClick::new(menu, minimap_width, Message::EditorRightClick).into()
    }

    /// Open files with unsaved changes (the dot in the sidebar).
    fn dirty_files(&self) -> HashSet<PathBuf> {
        let mut dirty: HashSet<PathBuf> = self
            .file_cache
            .iter()
            .filter(|(_, entry)| entry.dirty)
            .map(|(path, _)| path.clone())
            .collect();
        if let (true, Some(current)) = (self.is_dirty, &self.current_file) {
            dirty.insert(current.clone());
        }
        dirty
    }

    /// The rendered document, or the parser error when the JSON is invalid.
    fn preview_view(&self, dark: bool) -> Element<'_, Message> {
        match &self.preview {
            // `lazy` keeps the built widget tree until the revision changes:
            // rebuilding up to 20 000 JSON rows on every frame is wasteful.
            PreviewDoc::Markdown(blocks) => {
                let blocks = blocks.clone();
                // The wrapper gives `lazy` (which reports a shrink size) the
                // full pane.
                container(iced::widget::lazy(
                    (self.preview_rev, dark, self.config.high_contrast),
                    move |_| crate::preview::markdown::view(&blocks, dark),
                ))
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
            }
            PreviewDoc::Json(value) => {
                let (value, collapsed) = (value.clone(), self.json_collapsed.clone());
                container(iced::widget::lazy(
                    (self.preview_rev, dark, self.config.high_contrast),
                    move |_| crate::preview::json::view(&value, &collapsed, dark),
                ))
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
            }
            PreviewDoc::JsonError(err) => container(
                column![
                    text(t!("preview.json_error").to_string())
                        .size(13)
                        .color(se_theme::error_color(dark)),
                    text(err.clone())
                        .size(12)
                        .font(se_theme::MONO)
                        .color(se_theme::muted_text(dark)),
                ]
                .spacing(6),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .padding([16, 20])
            .into(),
            PreviewDoc::None => container(Space::new().height(0))
                .width(Length::Fill)
                .height(Length::Fill)
                .into(),
        }
    }
}

#[path = "app/file_io.rs"]
mod file_io;
use file_io::*;
#[path = "app/bookmark_panel.rs"]
mod bookmark_panel;
mod compare;
#[path = "app/completion.rs"]
mod completion;
#[path = "app/everywhere.rs"]
mod everywhere;
pub(crate) use completion::Floating;
#[path = "app/finder.rs"]
mod finder;
#[path = "app/lint.rs"]
mod lint;
#[path = "app/plugin_grid.rs"]
mod plugin_grid;
#[path = "app/plugins.rs"]
mod plugins;
#[path = "app/project_ops.rs"]
mod project_ops;
#[path = "app/results.rs"]
mod results;
mod spelling;
#[path = "app/split.rs"]
mod split;
#[path = "app/tab_groups.rs"]
mod tab_groups;
#[path = "app/views.rs"]
mod views;
mod welcome;
pub(crate) use views::resize_edge;

#[cfg(test)]
#[path = "app/tests.rs"]
mod tests;

/// What a tab is called: the file name, or "Untitled N".
pub fn display_name(path: &Path) -> String {
    if is_diff(path) {
        // diff://3/title
        let rest = path.to_str().unwrap_or("").trim_start_matches("diff://");
        let title = rest.split_once('/').map_or(rest, |(_, t)| t);
        format!("Δ {title}")
    } else if is_db(path) {
        // db://3/schema.table
        let rest = path.to_str().unwrap_or("").trim_start_matches("db://");
        let title = rest.split_once('/').map_or(rest, |(_, t)| t);
        format!("▦ {title}")
    } else if let Some(title) = crate::plugins::tab_title(path) {
        title
    } else if is_merge(path) {
        // merge://3/dir/file.rs
        let rest = path.to_str().unwrap_or("").trim_start_matches("merge://");
        let file = rest.split_once('/').map_or(rest, |(_, f)| f);
        format!("⇄ {}", file.rsplit('/').next().unwrap_or(file))
    } else if path.to_str().is_some_and(|s| s.starts_with("rev://")) {
        // rev://4/file.rs @ 1a2b3c4d: a file as an older commit had it
        let rest = path.to_str().unwrap_or("").trim_start_matches("rev://");
        let title = rest.split_once('/').map_or(rest, |(_, t)| t);
        format!("⟲ {title}")
    } else if is_untitled(path) {
        let n = path
            .to_str()
            .unwrap_or("")
            .trim_start_matches("untitled://");
        format!("{} {}", t!("app.untitled"), n)
    } else {
        path.file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "?".to_string())
    }
}

fn file_label(path: &Path) -> String {
    display_name(path)
}

/// `display_name` of each path, with the parent folder added to the ones
/// that would otherwise read the same.
pub fn unique_labels(paths: &[PathBuf]) -> Vec<String> {
    let names: Vec<String> = paths.iter().map(|p| display_name(p)).collect();
    paths
        .iter()
        .zip(&names)
        .map(|(path, name)| {
            let twins = names.iter().filter(|n| *n == name).count();
            let parent = path
                .parent()
                .and_then(|d| d.file_name())
                .map(|d| d.to_string_lossy().into_owned());
            match parent {
                Some(parent) if twins > 1 && !is_untitled(path) => format!("{name} — {parent}"),
                _ => name.clone(),
            }
        })
        .collect()
}

/// How a confirmation looks: the main action red (and a picto if it removes
/// something), or the accent.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Confirm {
    /// Red, as for a loss (changes thrown away, a quit).
    Danger,
    /// Red with the trash: something is deleted.
    Delete,
    /// The accent: no loss.
    Primary,
}

/// A small modal asking to confirm a destructive action (or cancel).
fn confirm_card(
    dark: bool,
    title: String,
    message: String,
    confirm_label: String,
    confirm: Message,
) -> Element<'static, Message> {
    confirm_card_with(dark, title, message, None, confirm_label, confirm)
}

/// The same for something that is deleted.
fn confirm_delete_card(
    dark: bool,
    title: String,
    message: String,
    confirm_label: String,
    confirm: Message,
) -> Element<'static, Message> {
    confirm_card_kind(
        dark,
        title,
        message,
        None,
        confirm_label,
        confirm,
        Confirm::Delete,
    )
}

/// A confirmation card, with something (a checkbox) between the text and the
/// buttons when `extra` is given.
fn confirm_card_with(
    dark: bool,
    title: String,
    message: String,
    extra: Option<Element<'static, Message>>,
    confirm_label: String,
    confirm: Message,
) -> Element<'static, Message> {
    confirm_card_kind(
        dark,
        title,
        message,
        extra,
        confirm_label,
        confirm,
        Confirm::Danger,
    )
}

fn confirm_card_kind(
    dark: bool,
    title: String,
    message: String,
    extra: Option<Element<'static, Message>>,
    confirm_label: String,
    confirm: Message,
    kind: Confirm,
) -> Element<'static, Message> {
    use crate::popin::{action, Action, Head};
    se_theme::in_popin(|| {
        let p = se_theme::palette(dark);
        let (icon, danger) = match kind {
            Confirm::Delete => ("trash-2", true),
            Confirm::Danger => ("triangle-alert", true),
            Confirm::Primary => ("shield-check", false),
        };
        let mut head = Head::new(icon, title);
        if danger {
            head = head.danger();
        }
        let mut body = column![text(message)
            .size(13)
            .color(se_theme::secondary(dark))
            .shaping(iced::widget::text::Shaping::Advanced)]
        .spacing(14)
        .width(Length::Fill);
        let _ = p;
        if let Some(extra) = extra {
            body = body.push(extra);
        }
        crate::popin::card(
            dark,
            crate::popin::WIDTH_CONFIRM,
            head,
            Some(Message::CloseOverlay),
            body.into(),
            Some((
                vec![],
                vec![
                    action(
                        dark,
                        t!("confirm.cancel").to_string(),
                        Some(Message::CloseOverlay),
                        Action::Cancel,
                    ),
                    action(
                        dark,
                        confirm_label,
                        Some(confirm),
                        if kind == Confirm::Primary {
                            Action::Primary
                        } else {
                            Action::Danger
                        },
                    ),
                ],
            )),
        )
    })
}

/// A 1px horizontal hairline.
fn rule<'a>(dark: bool) -> Element<'a, Message> {
    container(Space::new().height(1))
        .width(Length::Fill)
        .style(se_theme::rule(dark))
        .into()
}

/// A hairline between two panels side by side.
fn vrule<'a>(dark: bool) -> Element<'a, Message> {
    container(Space::new().width(1).height(Length::Fill))
        .style(se_theme::rule(dark))
        .into()
}

/// A folder for the toolbar: the home directory shown as `~`, with a
/// trailing slash.
fn short_dir(dir: &Path) -> String {
    let shown = match dirs::home_dir()
        .and_then(|home| dir.strip_prefix(&home).ok().map(Path::to_path_buf))
    {
        Some(rest) if rest.as_os_str().is_empty() => "~".to_string(),
        Some(rest) => format!("~/{}", rest.display()),
        None => dir.display().to_string(),
    };
    let shown = if shown.ends_with('/') {
        shown
    } else {
        format!("{shown}/")
    };
    // The toolbar has one line: keep the end of a long path (the folder the
    // file is in matters most).
    let count = shown.chars().count();
    if count > MAX_DIR_CHARS {
        let tail: String = shown.chars().skip(count - (MAX_DIR_CHARS - 1)).collect();
        format!("…{tail}")
    } else {
        shown
    }
}

/// Longest folder path shown in the toolbar, in characters.
const MAX_DIR_CHARS: usize = 56;

/// Where the cursor was: an entry of the Back / Forward history.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Place {
    path: PathBuf,
    line: usize,
    col: usize,
}

const MAX_PLACES: usize = 100;

/// The pattern search for a definition, off the UI thread (it may read many
/// files).
async fn patterns(
    word: String,
    path: PathBuf,
    text: String,
    line: usize,
    col: usize,
    open: Vec<crate::definition::Source>,
) -> crate::lsp::Outcome {
    use crate::lsp::Outcome;
    let found = tokio::task::spawn_blocking(move || {
        let current = (path.clone(), Some(text));
        crate::definition::locate(&word, &current, line, col, &open)
    })
    .await
    .ok()
    .flatten();
    found.map_or(Outcome::NotFound, Outcome::Found)
}

fn rename_field_id() -> iced::widget::Id {
    iced::widget::Id::new("rename-field")
}

fn goto_line_id() -> iced::widget::Id {
    iced::widget::Id::new("goto-line")
}

// ─── Helpers ────────────────────────────────────────────────────────────────

pub fn untitled_path(n: u32) -> PathBuf {
    PathBuf::from(format!("untitled://{}", n))
}

/// The question asked before moving `source` into `dir`.
fn move_message(source: &Path, dir: &Path) -> String {
    let (n, d) = (file_label(source), file_label(dir));
    t!("projects.move_message", name = n.as_str(), dir = d.as_str()).to_string()
}

/// Whether `source` may be moved into the folder `dir`: not into itself,
/// what is inside it, or the folder it already is in.
fn movable(source: &Path, dir: &Path) -> bool {
    !dir.starts_with(source) && source.parent() != Some(dir)
}

/// Opens the system's file manager on a folder, or on the folder of a
/// file.
fn reveal_in_file_manager(path: &Path) -> std::io::Result<()> {
    let dir = if path.is_dir() {
        path
    } else {
        path.parent().unwrap_or(path)
    };
    let program = if cfg!(target_os = "macos") {
        "open"
    } else if cfg!(windows) {
        "explorer"
    } else {
        "xdg-open"
    };
    std::process::Command::new(program)
        .arg(dir)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map(drop)
}

/// Whether a tab can be shown in a pane of the split: not a diff or a merge.
pub fn splittable(path: &Path) -> bool {
    !is_virtual(path)
}

/// A tab that is not a file: an untitled buffer, or a diff.
/// Bounds of the file list's width.
/// The height the bottom panels start with, and the least they can take.
pub(crate) const PANEL_HEIGHT: f32 = 340.0;
pub(crate) const PANEL_MIN_HEIGHT: f32 = 160.0;
/// Where the list starts: the gap, the bar of panels, and a gap.
const SIDEBAR_LEFT: f32 = ACTIVITY_WIDTH + 1.0;
/// The strip at the right of the list that grabs to resize it, before the
/// hairline.
const SIDEBAR_GRIP: f32 = 4.0;
/// Width of the bar of panels.
pub(crate) const ACTIVITY_WIDTH: f32 = 44.0;
const SIDEBAR_MIN: f32 = 160.0;
const SIDEBAR_MAX: f32 = 700.0;
pub(crate) const RIGHT_MIN: f32 = 280.0;
pub(crate) const RIGHT_MAX: f32 = 900.0;

/// A path for a person: the home folder written `~`.
pub fn display_path(path: &Path) -> String {
    match dirs::home_dir().and_then(|home| path.strip_prefix(&home).ok().map(Path::to_path_buf)) {
        Some(rest) => format!("~/{}", rest.display()),
        None => path.display().to_string(),
    }
}

/// Whether git tracks a file (it is in a repository and in its index).
/// Deletes a file from the disk (to the trash when there is one).
async fn delete_file(path: PathBuf) -> Result<(), String> {
    let target = path.clone();
    tokio::task::spawn_blocking(move || crate::fsutil::trash_or_remove(&target))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| format!("{}: {e}", path.display()))
}

pub fn is_untitled(path: &Path) -> bool {
    path.to_str()
        .map(|s| {
            s.starts_with("untitled://")
                || s.starts_with("diff://")
                || s.starts_with("merge://")
                || s.starts_with("rev://")
                || s.starts_with("db://")
                || s.starts_with("plugin://")
        })
        .unwrap_or(false)
}

/// A tab that shows a diff (`diff://N/title`).
pub fn is_diff(path: &Path) -> bool {
    path.to_str().is_some_and(|s| s.starts_with("diff://"))
}

/// A tab where a conflict is settled (`merge://N/path`).
pub fn is_merge(path: &Path) -> bool {
    path.to_str().is_some_and(|s| s.starts_with("merge://"))
}

/// A tab that is not a file and is never saved or remembered.
pub fn is_virtual(path: &Path) -> bool {
    is_diff(path) || is_merge(path) || is_db(path) || is_plugin_tab(path)
}

/// A tab of a plugin (`plugin://<plugin>/<tab>/<title>`).
pub fn is_plugin_tab(path: &Path) -> bool {
    path.to_str().is_some_and(|s| s.starts_with("plugin://"))
}

/// A tab that shows a table of a database (`db://N/schema.table`).
pub fn is_db(path: &Path) -> bool {
    path.to_str().is_some_and(|s| s.starts_with("db://"))
}

/// Releases as a column: version and date, then each section and its changes.
fn changelog_list<'a>(dark: bool, entries: &[crate::changelog::Entry]) -> Element<'a, Message> {
    let p = se_theme::palette(dark);
    let mut list = column![]
        .spacing(6)
        .width(Length::Fill)
        .padding(iced::Padding {
            top: 0.0,
            right: 12.0,
            bottom: 0.0,
            left: 0.0,
        });
    for entry in entries {
        list = list.push(Space::new().height(8));
        let head = if entry.date.is_empty() {
            format!("v{}", entry.version)
        } else {
            format!("v{} · {}", entry.version, entry.date)
        };
        list = list.push(text(head).size(15).color(p.text));
        for (title, changes) in &entry.sections {
            list = list.push(text(title.clone()).size(12).color(p.accent));
            for change in changes {
                list = list.push(text(format!("•  {change}")).size(13).color(p.muted));
            }
        }
    }
    list.into()
}
