fn main() {
    let args: Vec<_> = std::env::args().collect();
    let result = if args.len() == 4 {
        koine_steam::faults::crash_probe(
            std::path::Path::new(&args[1]),
            std::path::Path::new(&args[2]),
            &args[3],
        )
    } else {
        Err("usage: koine-steam-crash-probe NEW_LAB_DATABASE MARKED_SYNTHETIC_ROOT registered|journaled".into())
    };
    if let Err(e) = result {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
