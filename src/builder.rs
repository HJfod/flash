
use std::{collections::{HashMap, HashSet}, path::{PathBuf, Path}, fs};
use clang::{EntityKind, Entity};
use strfmt::strfmt;

use crate::config::Config;

struct Builder<'a> {
    pub config: &'a Config,
    pub already_built: HashSet<String>,
}

impl<'a> Builder<'a> {
    pub fn new(config: &'a Config) -> Self {
        Self {
            config,
            already_built: HashSet::new(),
        }
    }
}

fn build_docs_recurse(builder: &mut Builder, entity: Entity, namespace: &PathBuf, file: &Path) {
    for entity in entity.get_children() {
        if entity.is_in_system_header() {
            continue;
        }
        let source_url;
        let header_url;
        if let Some(ref repo) = builder.config.repository {
            let branch = builder.config.branch.clone().unwrap_or("main".into());
            let header_tree = builder.config.header_tree
                .clone().map(|p| p + "/").unwrap_or("".into());
            let source_tree = builder.config.source_tree
                .clone().map(|p| p + "/").unwrap_or("".into());
            source_url = format!(
                "{}/tree/{}/{}{}",
                repo, branch, source_tree,
                entity.get_file()
                    .map(|f| f.get_path().to_str().unwrap().to_owned())
                    .unwrap_or("none".into())
            ).into();
            header_url = format!(
                "{}/tree/{}/{}{}",
                repo, branch, header_tree, file.to_str().unwrap()
            ).into();
        }
        else {
            source_url = None;
            header_url = None;
        }

        match entity.get_kind() {
            EntityKind::Namespace => {
                build_docs_recurse(
                    builder, entity,
                    &namespace.join(entity.get_name().unwrap_or("_anon_ns".into())),
                    file
                );
            },
            EntityKind::StructDecl | EntityKind::ClassDecl => {
                let Some(name) = entity.get_name() else {
                    continue;
                };
                builder.already_built.insert(name.clone());
                let target_path = namespace.join(name + ".html");
                if target_path.exists() {
                    continue;
                }

                let vars = HashMap::from([
                    (
                        "name".to_string(),
                        entity.get_name().unwrap()
                    ),
                    (
                        "description".into(),
                        entity.get_parsed_comment().map(|c| c.as_html()).unwrap_or(String::new())
                    ),
                    (
                        "source_url".into(),
                        source_url.unwrap_or("#".into())
                    ),
                    (
                        "header_url".into(),
                        header_url.unwrap_or("#".into())
                    ),
                ]);
                let data = strfmt(&builder.config.class_template(), &vars).unwrap();
                fs::create_dir_all(target_path.parent().unwrap()).unwrap();
                fs::write(&target_path, data).unwrap();
            },
            _ => {},
        }
    }
}

pub fn build_docs_for(config: &Config, input_dir: &PathBuf, output_dir: &PathBuf) {
    let clang = clang::Clang::new().unwrap();
    let index = clang::Index::new(&clang, false, false);
    for src in config.expanded_sources() {
        println!("Building docs for {}", src.to_str().unwrap());
        let unit = index.parser(&src).parse().unwrap();
        let mut builder = Builder::new(config);
        build_docs_recurse(
            &mut builder,
            unit.get_entity(),
            &output_dir,
            src.strip_prefix(&input_dir).unwrap_or(src.as_path())
        );
    }
}
