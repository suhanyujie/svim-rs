mod document;
mod editor;
mod row;
mod terminal;

fn main() {
    editor::Editor::default().run();
}
