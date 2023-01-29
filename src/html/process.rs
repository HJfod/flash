
use std::sync::Arc;

use swc::{try_with_handler, HandlerOpts, config::{JsMinifyOptions, Options}, BoolOrDataConfig};
use swc_common::{SourceMap, GLOBALS, FileName};
use swc_css::{parser::parse_file, ast::Stylesheet, codegen::{writer::basic::{BasicCssWriter, BasicCssWriterConfig}, CodeGenerator, CodegenConfig}};
use swc_css::codegen::Emit;

pub fn transpile_and_minify_js(input: String) -> Result<String, String> {
    // minify
    let cm = Arc::<SourceMap>::default();
    let c = swc::Compiler::new(cm.clone());

    GLOBALS.set(&Default::default(), || {
        try_with_handler(
            cm.clone(),
            HandlerOpts {
                ..Default::default()
            },
            |handler| {
                let mut fm = cm.new_source_file(FileName::Anon, input);
                let output = c.process_js_file(
                    fm.clone(),
                    handler,
                    &Options {
                        ..Default::default()
                    }
                )?;
                // idk if there's a better way to do this lol
                fm = cm.new_source_file(FileName::Anon, output.code);
                c.minify(
                    fm,
                    handler,
                    &JsMinifyOptions {
                        compress: BoolOrDataConfig::from_bool(true),
                        mangle: BoolOrDataConfig::from_bool(true),
                        ..Default::default()
                    },
                )
            }
        )
    })
    .map(|o| o.code)
    .map_err(|e| format!("{e}"))
}

pub fn transpile_and_minify_css(input: String) -> Result<String, String> {
    let cm = Arc::<SourceMap>::default();

    GLOBALS.set(&Default::default(), || {
        try_with_handler(
            cm.clone(),
            HandlerOpts {
                ..Default::default()
            },
            |handler| {
                let fm = cm.new_source_file(FileName::Anon, input);

                let mut errors = vec![];
                let mut res: Stylesheet = parse_file(&fm, Default::default(), &mut errors).unwrap();
                
                if !errors.is_empty() {
                    // i can't get these got damn errors to be converted into ones 
                    // i can return so just gonna straight-up panic then
                    panic!(
                        "{}",
                        errors.into_iter()
                            .map(|e| format!("{}", e.message()))
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                }
                for err in errors {
                    err.to_diagnostics(handler).emit();
                }
                
                swc_css::minifier::minify(&mut res, Default::default());

                let mut css_str = String::new();
                {
                    let wr = BasicCssWriter::new(&mut css_str, None, BasicCssWriterConfig::default());
                    let mut gen = CodeGenerator::new(wr, CodegenConfig { minify: true });
                    gen.emit(&res).unwrap();
                }
                Ok(css_str)
            }
        )
    })
    .map_err(|e| format!("{e}"))
}
