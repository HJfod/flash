
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

fn fmt_link(config: &Config, url: &str, text: &str) -> String {
    strfmt(&config.link_template, &HashMap::from([
        ("url".to_string(), url),
        ("text".to_string(), text),
    ])).unwrap()
}

fn build_docs_recurse(builder: &mut Builder, entity: Entity, namespace: &PathBuf, file: &Path) {
    for entity in entity.get_children() {
        if entity.is_in_system_header() {
            continue;
        }
        let source_link;
        let header_link;
        if let Some(ref tree) = builder.config.tree {
            let src_url = format!(
                "{}/{}",
                tree,
                entity.get_file()
                    .map(|f| f.get_path().to_str().unwrap().to_owned())
                    .unwrap_or("none".into())
            );
            let hdr_url = format!("{}/{}", tree, file.to_str().unwrap());

            header_link = fmt_link(builder.config, &hdr_url, "View Header").into();
            source_link = fmt_link(builder.config, &src_url, "View Source").into();
        }
        else {
            source_link = None;
            header_link = None;
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
                if !entity.is_definition() {
                    continue;
                }
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
                        entity.get_parsed_comment()
                            .map(|c| c.as_html())
                            .unwrap_or("<p>No Description Provided</p>".into())
                    ),
                    (
                        "source_link".into(),
                        source_link.unwrap_or("".into())
                    ),
                    (
                        "header_link".into(),
                        header_link.unwrap_or("".into())
                    ),
                ]);
                let data = strfmt(&builder.config.class_template, &vars)
                    .map_err(|e| format!(
                        "Unable to format class template: {}",
                        e
                    )).unwrap();
                fs::create_dir_all(target_path.parent().unwrap()).unwrap();
                fs::write(&target_path, data).unwrap();
            },
            _ => {},
        }
    }
}

pub fn build_docs_for(config: &Config, output_dir: &PathBuf) {
    // init clang
    let clang = clang::Clang::new().unwrap();
    let index = clang::Index::new(&clang, false, true);

    // Iterate headers
    for src in &config.headers {
        println!("Building docs for {}", src.to_str().unwrap());

        // Create parser
        let unit = index.parser(&src).parse().unwrap();
        let mut builder = Builder::new(config);

        // Build the doc files
        build_docs_recurse(
            &mut builder,
            unit.get_entity(),
            &output_dir,
            src.as_path()
        );
    }

    // Iterate sources
    for src in &config.sources {
        println!("Building docs for {}", src.to_str().unwrap());
        
        // Create parser
        let unit = index.parser(&src)
            .arguments(&[""])
            .parse().unwrap();
        let mut builder = Builder::new(config);

        // Build the doc files
        build_docs_recurse(
            &mut builder,
            unit.get_entity(),
            &output_dir,
            src.as_path()
        );
    }
}
