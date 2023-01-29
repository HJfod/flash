
use std::sync::Arc;

use lightningcss::stylesheet::{ParserOptions, PrinterOptions};
use swc::{try_with_handler, HandlerOpts, config::{JsMinifyOptions, Options}, BoolOrDataConfig};
use swc_common::{SourceMap, GLOBALS, FileName};

pub fn minify_html(input: String) -> Result<String, String> {
    String::from_utf8(minify_html::minify(
        input.as_bytes(),
        &minify_html::Cfg::default()
    )).map_err(|e| format!("{e}"))
}
 
pub fn minify_js(input: String) -> Result<String, String> {
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

pub fn minify_css(input: String) -> Result<String, String> {
    let sheet = lightningcss::stylesheet::StyleSheet::parse(
        &input, ParserOptions::default()
    ).map_err(|e| format!("{e}"))?;
    sheet.to_css(PrinterOptions {
        minify: true,
        ..PrinterOptions::default()
    }).map(|s| s.code).map_err(|e| format!("{e}"))
}
