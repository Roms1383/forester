mod statements;
#[cfg(test)]
mod tests;

use std::collections::{HashMap, HashSet};
use std::fmt::format;
use std::path::PathBuf;
use graphviz_rust::cmd::{CommandArg, Format};
use graphviz_rust::dot_generator::*;
use graphviz_rust::dot_structures::*;
use graphviz_rust::exec;
use graphviz_rust::printer::PrinterContext;
use itertools::Itertools;
use crate::gol::ast::{Call, ImportName, Key, Tree};
use crate::gol::GolError;
use crate::gol::project::{AliasName, File, FileName, Project, TreeName};
use crate::gol::visualizer::statements::ToStmt;


fn err(v: String) -> GolError {
    GolError::CompileError(v)
}

struct VizItem<'a> {
    call: &'a Call,
    parent_id: String,
    file_name: String,
}

#[derive(Default)]
struct State<'a> {
    gen: usize,
    pub stack: Vec<VizItem<'a>>,
}

impl<'a> State<'a> {
    fn next(&mut self) -> String {
        self.gen += 1;
        self.curr()
    }
    fn curr(&self) -> String {
        self.gen.to_string()
    }
    fn push(&mut self, call: &'a Call, parent_id: String, file: String) {
        self.stack.push(VizItem { call, parent_id, file_name: file })
    }
    fn pop(&mut self) -> Option<VizItem<'a>> {
        self.stack.pop()
    }
}

struct Visualizer<'a> {
    project: &'a Project,
}

#[derive(Default)]
struct ImportMap {
    aliases: HashMap<AliasName, TreeName>,
    trees: HashMap<TreeName, FileName>,
    files: HashSet<FileName>,
}

impl ImportMap {
    fn build(file: &File) -> Result<Self, GolError> {
        let mut map = ImportMap::default();

        for (file, items) in &file.imports {
            for item in items {
                match item {
                    ImportName::Id(v) => {
                        if map.trees.get(v).filter(|f| f != &file).is_some() {
                            return Err(err(format!("the import call {} is presented twice from several different files", v)));
                        }
                        if map.aliases.get(v).is_some() {
                            return Err(err(format!("the import call {} is presented as alias", v)));
                        }
                        map.trees.insert(v.to_string(), file.to_string());
                    }
                    ImportName::Alias(id, alias) => {
                        if map.aliases.get(alias).filter(|id| id != id).is_some() {
                            return Err(err(format!("the import alias {} is already defined for another call ", alias)));
                        }
                        map.aliases.insert(alias.to_string(), id.to_string());
                        map.trees.insert(id.to_string(), file.to_string());
                    }
                    ImportName::WholeFile => {
                        map.files.insert(file.to_string());
                    }
                }
            }
        }

        Ok(map)
    }
}

impl<'a> Visualizer<'a> {
    fn init_with_root(&self) -> Result<&Tree, GolError> {
        let (main_file, root) = &self.project.main;

        self.project.files
            .get(main_file)
            .ok_or(err(format!("no main file {}", main_file)))?
            .definitions.get(root)
            .ok_or(err(format!("no root {} in {}", root, main_file)))
    }
    fn get_file(&self, file: &String) -> Result<&File, GolError> {
        self.project.files.get(file.as_str()).ok_or(err(format!("unexpected error: the file {} not exists", &file)))
    }
    fn build_graph(&self) -> Result<Graph, GolError> {
        let (file, name) = &self.project.main;
        let mut graph = graph!(strict di id!(name));
        let root = self.init_with_root()?;
        let mut state = State::default();

        graph.add_stmt(root.to_stmt(state.next()));

        for call in &root.calls.elems {
            state.push(call, state.curr(), file.clone());
        }

        while let Some(item) = state.pop() {
            let VizItem { call, parent_id: parent, file_name } = item;
            let curr_file = &self.get_file(&file_name)?;
            let import_map = ImportMap::build(curr_file)?;

            let node = match call {
                Call::Lambda(tpe, calls) => {
                    let stmt = tpe.to_stmt(state.next());
                    for call in &calls.elems {
                        state.push(call, state.curr(), file.clone());
                    }
                    stmt
                }
                Call::Invocation(Key(name), args) => {
                    if let Some(tree) = curr_file.definitions.get(name) {
                        let stmt = tree.to_stmt(state.next());
                        for call in &tree.calls.elems {
                            state.push(call, state.curr(), file.clone());
                        }
                        stmt
                    } else {
                        let tree =
                            if let Some(file) = import_map.trees.get(name.as_str()) {
                                self.project
                                    .files
                                    .get(file)
                                    .and_then(|f| f.definitions.get(name))
                                    .ok_or(err(format!("the call {} can not be found in the file {} ", name, file)))?
                            } else if let Some(id) = import_map.aliases.get(name.as_str()) {
                                let file = &import_map.trees.get(id).ok_or(err(format!("the call {} is not presented", id)))?;

                                self.project
                                    .files
                                    .get(file.as_str())
                                    .and_then(|f| f.definitions.get(id))
                                    .ok_or(err(format!("the call {} can not be found in the file {} ", name, file)))?
                            } else {
                                &import_map
                                    .files
                                    .iter()
                                    .flat_map(|f| { self.project.files.get(file) })
                                    .find(|f| f.definitions.contains_key(file))
                                    .and_then(|f| f.definitions.get(file.as_str()))
                                    .ok_or(err(format!("the call {} can not be found", name)))?
                            };
                        let stmt = tree.to_stmt(state.next());
                        for call in &tree.calls.elems {
                            state.push(call, state.curr(), file.clone());
                        }
                        stmt
                    }
                }
                Call::Decorator(tpe, args, call) => {
                    let stmt = (tpe, args).to_stmt(state.next());
                    state.push(call.as_ref(), state.curr(), file.clone());
                    stmt
                }
            };
            let edge = stmt!(edge!(node_id!(parent) => node_id!(state.curr())));
            graph.add_stmt(node);
            graph.add_stmt(edge);
        }


        Ok(graph)
    }

    pub fn to_svg_file(&mut self, path: String) -> Result<String, GolError> {
        let mut g = self.build_graph()?;

        exec(
            g,
            &mut PrinterContext::default(),
            vec![
                Format::Svg.into(),
                CommandArg::Output(path),
            ],
        ).map_err(|e| GolError::VisualizationError(e.to_string()))
    }
}


impl<'a> Visualizer<'a> {
    pub fn new(project: &'a Project) -> Self {
        Self { project }
    }
}

