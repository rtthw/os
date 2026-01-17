//! # Compiler



pub fn run() {
    let config = interface::Config {
        opts: session::config::Options {
            crate_types: vec![session::config::CrateType::Cdylib],
            incremental: None, // TODO: Use incremental compilation.
            output_types: session::config::OutputTypes::new(&[(
                session::config::OutputType::Exe,
                Some(session::config::OutFileName::Real("doubler.so".into())),
            )]),
            cg: session::config::CodegenOptions {
                opt_level: "2".into(),
                panic: Some(rustc_target::spec::PanicStrategy::Abort),
                strip: session::config::Strip::Symbols,
                ..Default::default()
            },
            verbose: true,
            ..Default::default()
        },
        crate_cfg: Vec::new(),
        crate_check_cfg: Vec::new(),
        input: session::config::Input::Str {
            name: span::FileName::Custom("doubler.rs".into()),
            input: r#"
                #![no_std]

                #[unsafe(no_mangle)]
                pub extern "C" fn doubler(n: f32) -> f32 {
                    n * 2.0
                }

                #[panic_handler]
                fn panic(_info: &core::panic::PanicInfo) -> ! {
                    loop {}
                }
                "#
            .into(),
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
        let sess = &compiler.sess;
        let codegen_backend = &*compiler.codegen_backend;
        let krate = interface::passes::parse(sess);
        println!("{krate:?}\n");
        let linker = interface::create_and_enter_global_ctxt(&compiler, krate, |tcx| {
            for id in tcx.hir_free_items() {
                let item = tcx.hir_item(id);
                match item.kind {
                    hir::ItemKind::Fn { ident, .. } => {
                        let ty = tcx.type_of(item.hir_id().owner.def_id);
                        println!("{ident:?}:\t{ty:?}");
                    }
                    _ => {}
                }
            }

            interface::Linker::codegen_and_build_linker(tcx, codegen_backend)
        });

        linker.link(sess, codegen_backend);
    });
}
