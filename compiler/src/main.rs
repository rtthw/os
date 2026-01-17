#![feature(rustc_private)]

extern crate rustc_driver;
extern crate rustc_error_codes;
extern crate rustc_errors;
extern crate rustc_hir as hir;
extern crate rustc_interface as interface;
extern crate rustc_session as session;
extern crate rustc_span as span;



fn main() {
    let config = interface::Config {
        opts: session::config::Options {
            incremental: None, // TODO: Use incremental compilation.
            ..Default::default()
        },
        crate_cfg: Vec::new(),
        crate_check_cfg: Vec::new(),
        input: session::config::Input::Str {
            name: span::FileName::Custom("lib.rs".into()),
            input: "fn doubler(n: f32) -> f32 { n * 2.0 }".into(),
        },
        output_dir: None,
        output_file: None,
        file_loader: None,
        locale_resources: rustc_driver::DEFAULT_LOCALE_RESOURCES.to_owned(),
        lint_caps: Default::default(),
        psess_created: None,
        register_lints: None,
        override_queries: None,
        registry: rustc_errors::registry::Registry::new(rustc_errors::codes::DIAGNOSTICS),
        make_codegen_backend: None,
        extra_symbols: Vec::new(),
        ice_file: None,
        hash_untracked_state: None,
        using_internal_features: &rustc_driver::USING_INTERNAL_FEATURES,
    };
    interface::run_compiler(config, |compiler| {
        let krate = interface::passes::parse(&compiler.sess);
        println!("{krate:?}\n");
        interface::create_and_enter_global_ctxt(&compiler, krate, |tcx| {
            for id in tcx.hir_free_items() {
                let item = tcx.hir_item(id);
                match item.kind {
                    hir::ItemKind::Fn { ident, .. } => {
                        let ty = tcx.type_of(item.hir_id().owner.def_id);
                        println!("{ident:?}:\t{ty:?}");
                    }
                    _ => (),
                }
            }
        });
    });
}
